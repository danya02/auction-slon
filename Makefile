all: build-front-admin build-front-user build-backend run-backend

build-front-admin:
	trunk build frontend/admin/index.html --filehash false --public-url "/admin"

build-front-user:
	trunk build frontend/user/index.html --filehash false --public-url "/"

build-backend:
	cargo build --bin backend

run-backend:
	RUST_LOG=debug cargo run --bin backend