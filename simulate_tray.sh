#!/bin/bash
# simulate_tray.sh - Debug tray icon responsiveness using automated clicks

# 1. Start application in background if not running
if ! pgrep -x "matrix-overlay" > /dev/null; then
    echo "Starting matrix-overlay..."
    cargo run --release &
    sleep 5
fi

# 2. Try to find the dummy window ID
WINDOW_ID=$(xdotool search --name "matrix-overlay" | head -n 1)

if [ -z "$WINDOW_ID" ]; then
    echo "Warning: Could not find matrix-overlay window ID. Using fallback search."
fi

# 3. Tray location hunting (Pop!_OS/GNOME Top-Right default)
# We test a grid around likely tray coordinates
COORDS=(
    "1880 15" "1850 15" "1820 15" "1790 15"
    "1760 15" "1730 15" "1700 15" "1670 15"
    "1880 35" "1850 35" "1820 35" "1790 35"
)

echo "Starting automated click simulation on tray area..."
echo "Monitoring /tmp/matrix_overlay_logs/matrix_overlay.log for 'TRAY CLICK'..."

# Start tailing in background to see clicks immediately
tail -f /tmp/matrix_overlay_logs/matrix_overlay.log | grep --line-buffered "TRAY CLICK" &
TAIL_PID=$!

for coord in "${COORDS[@]}"; do
    X=$(echo $coord | cut -d' ' -f1)
    Y=$(echo $coord | cut -d' ' -f2)
    echo "Simulating Left-Click at $X, $Y..."
    xdotool mousemove $X $Y click 1
    sleep 1
    echo "Simulating Right-Click at $X, $Y..."
    xdotool mousemove $X $Y click 3
    sleep 2
done

# 4. Try clicking the dummy window if found
if [ ! -z "$WINDOW_ID" ]; then
    echo "Trying to click dummy window ID: $WINDOW_ID..."
    xdotool windowactivate $WINDOW_ID
    xdotool click 1
    sleep 1
fi

echo "Simulation complete."
kill $TAIL_PID
echo "Check /tmp/matrix_overlay_logs/matrix_overlay.log for results."
