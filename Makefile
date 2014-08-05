GOPATH=`pwd`/../../../../
GO=GOPATH=$(GOPATH) go

OK_COLOR=\033[32;01m
NO_COLOR=\033[0m

build:
	@echo "$(OK_COLOR)==>$(NO_COLOR) Installing dependencies"
	@$(GO) get -v ./...
	@echo "$(OK_COLOR)==>$(NO_COLOR) Compiling"
	@$(GO) build -v ./...

run: build
	@echo "$(OK_COLOR)==>$(NO_COLOR) Running"
	./../../../../bin/sentry-relay

.PHONY: build run
