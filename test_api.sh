#!/bin/bash

# Usage: ./test_api.sh <audio_file>

AUDIO_FILE=$1
API_URL="http://127.0.0.1:8080"

if [ -z "$AUDIO_FILE" ]; then
    echo "Usage: $0 <audio_file>"
    exit 1
fi

echo "Uploading \"$AUDIO_FILE\"..."
RESPONSE=$(curl -s -F "file=@$AUDIO_FILE" $API_URL/upload)
TASK_ID=$(echo $RESPONSE | grep -o '"task_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$TASK_ID" ]; then
    echo "Upload failed. Response: $RESPONSE"
    exit 1
fi

echo "Task ID: $TASK_ID"

while true; do
    STATUS_RES=$(curl -s $API_URL/status/$TASK_ID)
    STATUS=$(echo $STATUS_RES | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    
    echo "Status: $STATUS"
    
    if [ "$STATUS" == "Completed" ]; then
        echo "Transcription completed!"
        break
    elif [ "$STATUS" == "Failed" ]; then
        echo "Transcription failed!"
        echo $STATUS_RES
        exit 1
    fi
    
    sleep 2
done

echo "Downloading subtitle..."
curl -s -O -J $API_URL/download/$TASK_ID
echo "Done. Check the downloaded .srt file."
