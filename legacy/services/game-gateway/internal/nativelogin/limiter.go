package nativelogin

import (
	"sync"
	"time"
)

// windowLimiter is a bounded in-process fixed-window counter. A key allows limit hits within
// window from its first hit. When the key table is full even after dropping expired windows it
// refuses every new key (fail closed) rather than growing without bound. A refused hit is not
// counted and changes no other state (§10).
type windowLimiter struct {
	mu      sync.Mutex
	limit   int
	window  time.Duration
	maxKeys int
	now     func() time.Time
	entries map[string]limiterEntry
}

type limiterEntry struct {
	start time.Time
	count int
}

func newWindowLimiter(limit int, window time.Duration, maxKeys int, now func() time.Time) *windowLimiter {
	return &windowLimiter{
		limit:   limit,
		window:  window,
		maxKeys: maxKeys,
		now:     now,
		entries: map[string]limiterEntry{},
	}
}

// allow records one hit for key, or returns the whole seconds until it may be retried.
func (l *windowLimiter) allow(key string) (bool, int) {
	l.mu.Lock()
	defer l.mu.Unlock()

	now := l.now()
	entry, exists := l.entries[key]
	if exists && now.Sub(entry.start) >= l.window {
		exists = false
	}
	if !exists {
		if _, stale := l.entries[key]; !stale && len(l.entries) >= l.maxKeys {
			l.sweep(now)
			if len(l.entries) >= l.maxKeys {
				return false, retrySeconds(l.window)
			}
		}
		l.entries[key] = limiterEntry{start: now, count: 1}
		return true, 0
	}
	if entry.count >= l.limit {
		return false, retrySeconds(entry.start.Add(l.window).Sub(now))
	}
	entry.count++
	l.entries[key] = entry
	return true, 0
}

func (l *windowLimiter) sweep(now time.Time) {
	for key, entry := range l.entries {
		if now.Sub(entry.start) >= l.window {
			delete(l.entries, key)
		}
	}
}

func retrySeconds(remaining time.Duration) int {
	seconds := int((remaining + time.Second - 1) / time.Second)
	if seconds < 1 {
		return 1
	}
	return seconds
}
