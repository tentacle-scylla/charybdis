package charybdis

import (
	"context"
	"errors"
)

var (
	// ErrSessionClosed is returned when operating on a closed session
	ErrSessionClosed = errors.New("charybdis: session is closed")

	// ErrBenchmarkCancelled is returned when a benchmark is cancelled
	ErrBenchmarkCancelled = errors.New("charybdis: benchmark cancelled")

	// ErrInvalidConfig is returned when configuration is invalid
	ErrInvalidConfig = errors.New("charybdis: invalid configuration")

	// ErrConnectionFailed is returned when connection to ScyllaDB fails
	ErrConnectionFailed = errors.New("charybdis: connection failed")

	// ErrScriptError is returned when the Rune script has errors
	ErrScriptError = errors.New("charybdis: script validation failed")
)

// Session represents a benchmark session.
// It holds the configuration and state for running benchmarks.
// A session can be reused for multiple benchmark runs.
type Session struct {
	config   BenchmarkConfig
	workload WorkloadConfig
	ptr      uintptr // Opaque pointer to Rust session (0 if not created)
	closed   bool
}

// NewSession creates a new benchmark session with the given configuration.
// The session must be closed with Close() when done.
func NewSession(config BenchmarkConfig, workload WorkloadConfig) (*Session, error) {
	if err := validateConfig(config); err != nil {
		return nil, err
	}

	s := &Session{
		config:   config,
		workload: workload,
	}

	// Create the underlying Rust session
	if err := s.init(); err != nil {
		return nil, err
	}

	return s, nil
}

// Run executes the benchmark with the given context and progress callback.
// The callback is optional and can be nil if progress updates are not needed.
// The context can be used to cancel the benchmark.
func (s *Session) Run(ctx context.Context, callback ProgressCallback) (*BenchmarkResults, error) {
	if s.closed {
		return nil, ErrSessionClosed
	}

	return s.run(ctx, callback)
}

// Cancel cancels a running benchmark.
// This is safe to call from any goroutine, including from within the progress callback.
// If the benchmark is not running, this is a no-op.
func (s *Session) Cancel() {
	if s.closed || s.ptr == 0 {
		return
	}
	s.cancel()
}

// Close releases resources associated with the session.
// After Close is called, the session cannot be used again.
func (s *Session) Close() error {
	if s.closed {
		return nil
	}
	s.closed = true
	s.free()
	return nil
}

// ValidateScript validates a Rune script without running it.
// Returns nil if the script is valid, or a list of errors.
func ValidateScript(script string) ([]ScriptError, error) {
	return validateScript(script)
}

// BuiltinWorkloads returns the list of built-in workload templates.
func BuiltinWorkloads() ([]WorkloadTemplate, error) {
	return builtinWorkloads()
}

// Version returns the charybdis library version.
func Version() string {
	return version()
}

// validateConfig performs basic validation on the benchmark config.
func validateConfig(config BenchmarkConfig) error {
	if len(config.Hosts) == 0 {
		return errors.New("charybdis: at least one host is required")
	}
	if config.Duration < 0 {
		return errors.New("charybdis: duration cannot be negative")
	}
	if config.Warmup < 0 {
		return errors.New("charybdis: warmup cannot be negative")
	}
	if config.Threads < 0 {
		return errors.New("charybdis: threads cannot be negative")
	}
	if config.Concurrency < 0 {
		return errors.New("charybdis: concurrency cannot be negative")
	}
	if config.RateLimit < 0 {
		return errors.New("charybdis: rate_limit cannot be negative")
	}
	return nil
}
