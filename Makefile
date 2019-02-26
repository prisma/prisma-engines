build-image:
	docker build -t prismagraphql/rust-build:latest .buildkite
	docker push prismagraphql/rust-build:latest
