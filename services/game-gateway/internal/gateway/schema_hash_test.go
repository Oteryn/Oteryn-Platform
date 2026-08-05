package gateway

import (
	"crypto/sha256"
	"fmt"
	"os"
	"testing"
)

func TestCanonicalNativeSchemaSHA256(t *testing.T) {
	data, err := os.ReadFile("../../../../docs/contracts/oteryn_native_gameplay_v1.proto")
	if err != nil {
		t.Fatalf("read canonical schema: %v", err)
	}

	actual := fmt.Sprintf("%x", sha256.Sum256(data))
	if actual != canonicalNativeSchemaSHA256 {
		t.Fatalf("canonical native schema sha256 changed: got %s want %s", actual, canonicalNativeSchemaSHA256)
	}
}
