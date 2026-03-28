.PHONY: dev stop restart build check clean

PID_FILE := .tauri-dev.pid

dev:
	@if [ -f $(PID_FILE) ] && kill -0 $$(cat $(PID_FILE)) 2>/dev/null; then \
		echo "Already running (PID $$(cat $(PID_FILE))). Use 'make restart'."; \
	else \
		cargo tauri dev & echo $$! > $(PID_FILE); \
		echo "Started (PID $$(cat $(PID_FILE)))"; \
	fi

stop:
	@if [ -f $(PID_FILE) ] && kill -0 $$(cat $(PID_FILE)) 2>/dev/null; then \
		kill $$(cat $(PID_FILE)) 2>/dev/null; \
		rm -f $(PID_FILE); \
		echo "Stopped."; \
	else \
		echo "Not running."; \
		rm -f $(PID_FILE); \
	fi
	@-pkill -f "reeln-dock" 2>/dev/null || true
	@-pkill -f "cargo tauri dev" 2>/dev/null || true

restart: stop dev

build:
	cargo tauri build

check:
	npm run build
	cd src-tauri && cargo clippy -- -D warnings

clean:
	rm -rf dist node_modules/.vite
	cd src-tauri && cargo clean
	rm -f $(PID_FILE)
