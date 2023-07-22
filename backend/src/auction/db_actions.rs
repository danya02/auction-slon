use communication::{
    auction::state::{AuctionItem, Sponsorship, SponsorshipStatus},
    ItemState, ItemStateValue, Money, UserAccountData, UserAccountDataWithSecrets,
};
use sqlx::{query, SqlitePool};

pub async fn get_user_state(pool: &SqlitePool) -> anyhow::Result<Vec<UserAccountDataWithSecrets>> {
    let user_rows = query!("SELECT * FROM auction_user").fetch_all(pool).await?;
    let mut user_data = vec![];
    for row in user_rows {
        user_data.push(UserAccountDataWithSecrets {
            id: row.id,
            user_name: row.name,
            balance: row.balance as u32,
            login_key: row.login_key,
            sale_mode: row.sale_mode.into(),
            sponsorship_code: row.sponsorship_code,
        });
    }
    Ok(user_data)
}

pub async fn get_sponsorship_state(pool: &SqlitePool) -> anyhow::Result<Vec<Sponsorship>> {
    let sponsorship_rows = query!("SELECT * FROM sponsorship").fetch_all(pool).await?;
    Ok(sponsorship_rows
        .iter()
        .map(|row| Sponsorship {
            id: row.id,
            donor_id: row.donor_id,
            recepient_id: row.recepient_id,
            status: row.status.into(),
            balance_remaining: row.remaining_balance as Money,
        })
        .collect::<Vec<_>>())
}

pub async fn get_item_state(pool: &SqlitePool) -> anyhow::Result<Vec<ItemState>> {
    let item_rows = query!(r#"
        SELECT
            auction_item.id, auction_item.name, auction_item.initial_price, auction_item_sale.buyer_id, auction_item_sale.sale_price, auction_user.name AS username, auction_user.balance, auction_user.sale_mode, auction_user.sponsorship_code
        FROM auction_item
        LEFT OUTER JOIN auction_item_sale ON auction_item_sale.item_id = auction_item.id
        LEFT OUTER JOIN auction_user ON auction_item_sale.buyer_id = auction_user.id
        "#).fetch_all(pool).await?;
    let mut item_data = vec![];
    for row in item_rows {
        let item = AuctionItem {
            id: row.id,
            name: row.name,
            initial_price: row.initial_price as Money,
        };
        let state = match row.buyer_id {
            None => ItemStateValue::Sellable,
            Some(id) => ItemStateValue::AlreadySold {
                buyer: UserAccountData {
                    id,
                    user_name: row.username,
                    balance: row.balance as Money,
                    sale_mode: row.sale_mode.into(),
                    is_accepting_sponsorships: row.sponsorship_code.is_some(),
                },
                sale_price: row.sale_price.unwrap() as Money,
            },
        };
        item_data.push(ItemState { item, state });
    }
    Ok(item_data)
}

/// Transactionally apply an item sale:
///
/// - create a sale record for the item
/// - from each user's balance, subtract the contributed amount
/// - add a contribution record for each user
///
/// Panics if any user would have negative balance as a result of this.
/// Be sure to check balances previously.
pub async fn apply_contributions(
    pool: &SqlitePool,
    item_id: i64,
    buyer_id: i64,
    contributions: &[(i64, Money)],
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // Create a sale record
    let total_amount: Money = contributions.iter().map(|(_, b)| b).sum();
    query!(
        "INSERT INTO auction_item_sale (item_id, buyer_id, sale_price) VALUES (?,?,?)",
        item_id,
        buyer_id,
        total_amount
    )
    .execute(&mut tx)
    .await?;

    // To that sale record, add the contributions and subtract the amounts.
    for (uid, amt) in contributions.iter() {
        // TODO: If the contribution is zero, ignore it?
        // or keep it around as evidence of who took part?

        // Create contribution
        query!(
            "INSERT INTO sale_contribution (sale_id, user_id, amount) VALUES (?,?,?)",
            item_id,
            uid,
            amt
        )
        .execute(&mut tx)
        .await?;

        // Deduct amount
        query!(
            "UPDATE auction_user SET balance=balance-? WHERE id=?",
            amt,
            uid
        )
        .execute(&mut tx)
        .await?;

        // Deduct amount from sponsorship, if the sponsorship exists.
        let active = SponsorshipStatus::Active.to_db_val();
        query!("UPDATE sponsorship SET remaining_balance=remaining_balance-? WHERE status=? AND recepient_id=? AND donor_id=?",
            amt, active, buyer_id, uid).execute(&mut tx).await?;
    }

    tx.commit().await?;

    Ok(())
}
