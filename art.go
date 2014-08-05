package main

import "fmt"

const ARTWORK = `                _
 ___  ___ _ __ | |_ _ __ _   _
/ __|/ _ \ '_ \| __| '__| | | |
\__ \  __/ | | | |_| |  | |_| |
|___/\___|_| |_|\__|_|   \__, |
                         |___/

`

func printBanner(procs int, listen, upstream string) {
	fmt.Print(ARTWORK)
	fmt.Printf("Max Concurrency: %d\n", procs)
	fmt.Printf("Listen: %s\n", listen)
	fmt.Printf("Upstream: %s\n", upstream)
	fmt.Println("")
}
