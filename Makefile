all: fetch-bootstrap svg-size-fix build-front-admin build-front-user clean-bootstrap svg-size-unfix build-backend run-backend

fetch-bootstrap:
	wget -O bootstrap.min.css https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css

build-front-admin: fetch-bootstrap
	trunk build frontend/admin/index.html --release --filehash false --public-url "/admin"

build-front-user: fetch-bootstrap
	trunk build frontend/user/index.html --release --filehash false --public-url "/"

clean-bootstrap:
	rm -f ./bootstrap.min.css


# Strange hack: Inkscape cannot parse SVG size in ems, but we need that so that the icon shows up with a relative size
svg-size-fix:
	sed -i 's/width="24"/width="1.5em"/' slon-icon-filled.svg
	sed -i 's/height="24"/height="1.5em"/' slon-icon-filled.svg

svg-size-unfix:
	sed -i 's/width="1.5em"/width="24"/' slon-icon-filled.svg
	sed -i 's/height="1.5em"/height="24"/' slon-icon-filled.svg


build-backend:
	cargo build --bin backend

run-backend:
	RUST_LOG=debug cargo run --bin backend