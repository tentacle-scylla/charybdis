package charybdis

import (
	"testing"
	"time"
)

func TestValidateConfig(t *testing.T) {
	tests := []struct {
		name    string
		config  BenchmarkConfig
		wantErr bool
	}{
		{
			name: "valid config",
			config: BenchmarkConfig{
				Hosts:       []string{"127.0.0.1"},
				Port:        9042,
				Threads:     1,
				Concurrency: 100,
				Duration:    60 * time.Second,
			},
			wantErr: false,
		},
		{
			name: "empty hosts",
			config: BenchmarkConfig{
				Hosts: []string{},
			},
			wantErr: true,
		},
		{
			name: "negative duration",
			config: BenchmarkConfig{
				Hosts:    []string{"127.0.0.1"},
				Duration: -1 * time.Second,
			},
			wantErr: true,
		},
		{
			name: "negative warmup",
			config: BenchmarkConfig{
				Hosts:  []string{"127.0.0.1"},
				Warmup: -1 * time.Second,
			},
			wantErr: true,
		},
		{
			name: "negative threads",
			config: BenchmarkConfig{
				Hosts:   []string{"127.0.0.1"},
				Threads: -1,
			},
			wantErr: true,
		},
		{
			name: "negative concurrency",
			config: BenchmarkConfig{
				Hosts:       []string{"127.0.0.1"},
				Concurrency: -1,
			},
			wantErr: true,
		},
		{
			name: "negative rate limit",
			config: BenchmarkConfig{
				Hosts:     []string{"127.0.0.1"},
				RateLimit: -1,
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateConfig(tt.config)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateConfig() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestSessionClose(t *testing.T) {
	// Test that closing a session multiple times is safe
	s := &Session{
		config:   BenchmarkConfig{Hosts: []string{"127.0.0.1"}},
		workload: WorkloadConfig{Script: "test"},
		ptr:      0,
		closed:   false,
	}

	// First close should succeed
	err := s.Close()
	if err != nil {
		t.Errorf("Close() error = %v", err)
	}

	if !s.closed {
		t.Error("Session should be marked as closed")
	}

	// Second close should be a no-op
	err = s.Close()
	if err != nil {
		t.Errorf("Close() second call error = %v", err)
	}
}

func TestConsistencyConstants(t *testing.T) {
	// Test that consistency constants have expected values
	// Values must be snake_case to match Rust's serde serialization
	tests := []struct {
		consistency Consistency
		expected    string
	}{
		{ConsistencyAny, "any"},
		{ConsistencyOne, "one"},
		{ConsistencyTwo, "two"},
		{ConsistencyThree, "three"},
		{ConsistencyQuorum, "quorum"},
		{ConsistencyAll, "all"},
		{ConsistencyLocalOne, "local_one"},
		{ConsistencyLocalQuorum, "local_quorum"},
		{ConsistencyEachQuorum, "each_quorum"},
		{ConsistencySerial, "serial"},
		{ConsistencyLocalSerial, "local_serial"},
	}

	for _, tt := range tests {
		if string(tt.consistency) != tt.expected {
			t.Errorf("Consistency %s = %s, want %s", tt.consistency, string(tt.consistency), tt.expected)
		}
	}
}

func TestPhaseConstants(t *testing.T) {
	// Test that phase constants have expected values
	tests := []struct {
		phase    Phase
		expected string
	}{
		{PhasePreparing, "preparing"},
		{PhaseConnecting, "connecting"},
		{PhaseWarming, "warming"},
		{PhaseRunning, "running"},
		{PhaseCompleted, "completed"},
		{PhaseFailed, "failed"},
		{PhaseCancelled, "cancelled"},
	}

	for _, tt := range tests {
		if string(tt.phase) != tt.expected {
			t.Errorf("Phase %s = %s, want %s", tt.phase, string(tt.phase), tt.expected)
		}
	}
}

func TestBenchmarkConfigDefaults(t *testing.T) {
	config := BenchmarkConfig{
		Hosts: []string{"127.0.0.1"},
	}

	// Verify zero values for optional fields
	if config.Port != 0 {
		t.Error("Port should default to 0")
	}
	if config.Threads != 0 {
		t.Error("Threads should default to 0")
	}
	if config.Duration != 0 {
		t.Error("Duration should default to 0")
	}
}
