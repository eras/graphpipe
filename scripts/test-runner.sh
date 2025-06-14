#!/bin/bash

# Configuration
GRAPH_ENDPOINT="http://localhost:8080/graph/layout" # Adjust if your server runs on a different host/port
POLL_INTERVAL_SECONDS=1 # How often to poll the endpoint

LAST_CREATION_TIME=""

echo "Polling ${GRAPH_ENDPOINT} for creation_time changes. Will run test.sh if detected."
echo "Press Ctrl+C to stop."

while true; do
    CURRENT_CREATION_TIME=$(http GET "${GRAPH_ENDPOINT}" | jq -r '.creation_time')

    # Check if jq successfully extracted a value (it will be "null" if not found or error)
    if [ "$CURRENT_CREATION_TIME" = "null" ] || [ -z "$CURRENT_CREATION_TIME" ]; then
        echo "$(date): Warning: Could not get creation_time from ${GRAPH_ENDPOINT}. Retrying..."
    else
        # If LAST_CREATION_TIME is empty (first run) or different from CURRENT_CREATION_TIME
        if [ -z "$LAST_CREATION_TIME" ] || [ "$CURRENT_CREATION_TIME" != "$LAST_CREATION_TIME" ]; then
            if [ -n "$LAST_CREATION_TIME" ]; then
                echo "$(date): Creation time changed from ${LAST_CREATION_TIME} to ${CURRENT_CREATION_TIME}. Running test.sh..."
            else
                echo "$(date): Initial creation time: ${CURRENT_CREATION_TIME}. Monitoring for changes..."
            fi

            LAST_CREATION_TIME="$CURRENT_CREATION_TIME"

	    scripts/test.sh
        else
            #echo "$(date): Creation time (${CURRENT_CREATION_TIME}) unchanged. Polling again in ${POLL_INTERVAL_SECONDS} seconds."
	    :
        fi
    fi

    # Wait for the next poll
    sleep "$POLL_INTERVAL_SECONDS"
done
