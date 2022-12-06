basedir = "."

all: dev

# Prod
prod: build-prod run-prod

build-prod: front-prod back-prod

front-prod:
	cd $(basedir)/frontend; trunk build --release

back-prod:
	cd $(basedir)/backend/src/bin/prod; diesel migration run; cd ../../../; cargo build --bin prod --release

run-prod:
	cd $(basedir)/backend; cargo run --bin prod 

# Dev
dev: build-dev run-dev

build-dev: front-dev back-dev

front-dev:
	cd $(basedir)/frontend; trunk build

back-dev:
	cargo run --bin dev --features="dev"

run-dev:
	cd $(basedir)/backend; cargo run --bin dev --features="dev"
