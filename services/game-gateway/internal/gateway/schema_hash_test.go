package gateway

import (
	"crypto/sha256"
	"os"
	"testing"
)

func TestCaptureCanonicalNativeSchemaSHA256(t *testing.T) {
	data, err := os.ReadFile("../../../../docs/contracts/oteryn_native_gameplay_v1.proto")
	if err != nil {
		t.Fatalf("read canonical schema: %v", err)
	}

	digest := sha256.Sum256(data)
	t.Fatalf("canonical native schema sha256=%x", digest)
}
