#!/bin/bash
# forge.sh - Execute forgecode for task documentation in one-shot logged mode

# Configuration
TASK_NAME="$1"
LOG_DIR="$HOME/dotagents/logs"
FORECODE_BIN="/mnt/data1/time-2026/05-may/15/forgecode/target/debug/forge"
PROJECT_DIR="/mnt/data1/time-2026/05-may/15/forgecode"

# Create log directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Validate input
if [ -z "$TASK_NAME" ]; then
    echo "Error: No task name specified" >&2
    echo "Usage: $0 <task-name>" >&2
    exit 1
fi

# Check if task directory exists
TASK_DIR="$HOME/dotagents/tasks/$TASK_NAME"
if [ ! -d "$TASK_DIR" ]; then
    echo "Error: Task directory '$TASK_DIR' not found" >&2
    exit 1
fi

# Create log file
LOG_FILE="$LOG_DIR/${TASK_NAME}.log"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting task: $TASK_NAME" > "$LOG_FILE"

# Change to project directory
cd "$PROJECT_DIR" || {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Failed to enter $PROJECT_DIR" >> "$LOG_FILE"
    exit 1
}

# Read TASK.md file
if [ -f "$TASK_DIR/TASK.md" ]; then
    TASK_CONTENT=$(cat "$TASK_DIR/TASK.md")
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Executing task: $TASK_NAME" >> "$LOG_FILE"
    echo "Task content: $TASK_CONTENT" >> "$LOG_FILE"
    
    # Execute forgecode with prompt mode for the task
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running forgecode with prompt for $TASK_NAME" >> "$LOG_FILE"
    "$FORECODE_BIN" --prompt "$TASK_CONTENT" >> "$LOG_FILE" 2>&1
else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] TASK.md file not found in $TASK_DIR" >> "$LOG_FILE"
    exit 1
fi

# Check exit status
if [ $? -eq 0 ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Completed task: $TASK_NAME" >> "$LOG_FILE"
else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Task $TASK_NAME failed" >> "$LOG_FILE"
    exit 1
fi

# Capture telemetry for the running process
TELEMETRY_FILE="$LOG_DIR/${TASK_NAME}_telemetry.txt"
ps -p $$ -o pid,cmd >> "$TELEMETRY_FILE"
cmdline=$(cat /proc/$$/cmdline | tr '\0' ' ')
echo "Command line: $cmdline" >> "$TELEMETRY_FILE"
lsof -p $$ >> "$TELEMETRY_FILE" 2>&1
