// This is a fake Go main file to satisfy GoReleaser requirements
// The actual binary is built using Rust via the build-cross.sh script
package main

import "fmt"

func main() {
	fmt.Println("This should never be executed - FerroCP is built with Rust")
}
