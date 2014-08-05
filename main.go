package main

import (
	"flag"
	"log"
	"runtime"

	"github.com/getsentry/sentry-relay/api"
)

var (
	listen   = flag.String("listen", ":8080", "Address to bind to")
	procs    = flag.Int("c", runtime.NumCPU(), "Max concurrency")
	upstream = flag.String("upstream", "https://app.getsentry.com", "Upstream Sentry server")
)

func main() {
	printBanner(*procs, *listen, *upstream)
	flag.Parse()

	runtime.GOMAXPROCS(*procs)
	log.Fatal(api.ListenAndServe(*upstream, *listen))
}
