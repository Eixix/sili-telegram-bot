# TODO: backslashes vs. foward slashes!!
lint:
	python -m flake8 ./src/

test:
	pytest ./tests --verbose


.PHONY: lint test
