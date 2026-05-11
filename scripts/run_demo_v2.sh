#!/bin/bash
PROJECT_DIR="/Users/nonbangkok/Documents/Workspace/Project/Digital-Ghost"
CONFIG_FILE="$PROJECT_DIR/.yunaconfig"
SANDBOX="$PROJECT_DIR/sandbox_v2"
RESULTS="$PROJECT_DIR/results_v2"
MODEL="mistral:7b-instruct-v0.3-q4_K_M"

personalities=("yuna")

mkdir -p "$SANDBOX"
mkdir -p "$RESULTS"

for p in "${personalities[@]}"; do
    echo "Testing personality: $p"
    
    cat <<EOC > "$CONFIG_FILE"
[yuna]
personality = "$p"
ollama_model = "$MODEL"
ollama_url = "http://localhost:11434"
[watch]
paths = ["$SANDBOX"]
exclude = []
[behavior]
alias_injection = false
note_lifetime_minutes = 10080
[limits]
max_cpu_percent = 0.5
cooldown_seconds = 0
EOC

    rm -rf ~/.yuna/yuna.db
    "$PROJECT_DIR/target/debug/yuna" &
    YUNA_PID=$!
    sleep 5
    
    # Trigger Cleaning (Wait a bit between touch and rm)
    for i in {1..15}; do touch "$SANDBOX/junk_$i.tmp"; done
    sleep 2
    rm "$SANDBOX/junk_"*.tmp
    
    # Trigger Midnight Worker
    NOW=$(date +%s)
    DAY_START=$((NOW - (NOW % 86400)))
    sqlite3 ~/.yuna/yuna.db "INSERT INTO events (path, kind, timestamp) VALUES ('$SANDBOX/main.rs', 'modify', $((DAY_START + 100)));"
    sqlite3 ~/.yuna/yuna.db "INSERT INTO events (path, kind, timestamp) VALUES ('$SANDBOX/main.rs', 'modify', $((DAY_START + 15000)));"
    touch "$SANDBOX/trigger.rs"
    
    # Wait for 2 notes
    for i in {1..60}; do
        count=$("$PROJECT_DIR/target/debug/yunactl" status | grep "unread_notes" | awk '{print $2}')
        if [ "$count" -ge 2 ]; then
            echo "Found $count notes for $p"
            break
        fi
        sleep 2
    done
    
    mkdir -p "$RESULTS/$p"
    mv "$SANDBOX"/*.yuna "$RESULTS/$p/" 2>/dev/null
    
    kill $YUNA_PID
    wait $YUNA_PID 2>/dev/null
    rm -rf "$SANDBOX"/*
done
