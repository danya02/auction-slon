// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Nullable<Integer>,
        name -> Text,
        passcode -> Text,
        role -> Integer,
    }
}
