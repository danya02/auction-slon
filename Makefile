all: fetch-bootstrap build-front-admin build-front-user clean-bootstrap build-backend run-backend

fetch-bootstrap:
	wget -O bootstrap.min.css https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css

build-front-admin: fetch-bootstrap
	trunk build frontend/admin/index.html --filehash false --public-url "/admin"

build-front-user: fetch-bootstrap
	trunk build frontend/user/index.html --filehash false --public-url "/"

clean-bootstrap:
	rm -f ./bootstrap.min.css

build-backend:
	cargo build --bin backend

run-backend:
	RUST_LOG=debug cargo run --bin backend