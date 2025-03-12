#!/bin/bash

SERVER_IP="172.25.223.120"
SERVER_PORT="8080"
BASE_URL="http://${SERVER_IP}:${SERVER_PORT}"

# Generate a random GUID for the player
PLAYER_ID=$(uuidgen | tr -d '-')

echo "===== Friends Connect - Player 1 Script ====="
echo
echo "This script will walk you through the Player 1 flow for testing"
echo "the friends-connect application."
echo
echo "Generated player ID: ${PLAYER_ID}"
echo
read -p "Press enter to start..."

echo
echo "Step 1: Creating a new connection for Player 1..."
echo
echo "Running command:"
echo "curl -X POST ${BASE_URL}/connections -H \"Content-Type: application/json\" -d '{\"player_id\":\"${PLAYER_ID}\"}'"
echo

RESP=$(curl -s -X POST "${BASE_URL}/connections" -H "Content-Type: application/json" -d "{\"player_id\":\"${PLAYER_ID}\"}")

echo "Response: $RESP"

# Extract connection_id
CONNECTION_ID=$(echo $RESP | grep -o '"id":"[^"]*"' | head -1 | cut -d '"' -f 4)

# Extract link_id
LINK_ID=$(echo $RESP | grep -o '"link_id":"[^"]*"' | head -1 | cut -d '"' -f 4)

echo
echo "Connection Created!"
echo "Connection ID: $CONNECTION_ID"
echo
echo "LINK TO SHARE: ${BASE_URL}/connections/link/${LINK_ID}/join"
echo
echo "Now in your other terminal, run the Player 2 script with this link_id:"
echo "${LINK_ID}"
echo
read -p "After running the Player 2 script to the first pause, press enter to continue..."

echo
echo "Step 2: Checking for notifications (Player 2 should have joined)..."
echo
echo "Running command:"
echo "curl -X GET ${BASE_URL}/players/${PLAYER_ID}/notifications"
echo

curl -s -X GET "${BASE_URL}/players/${PLAYER_ID}/notifications"

echo
read -p "Press enter to continue..."

echo
echo "Step 3: Sending a message to Player 2..."
echo
echo "Running command:"
echo "curl -X POST ${BASE_URL}/connections/${CONNECTION_ID}/messages -H \"Content-Type: application/json\" -d '{\"player_id\":\"${PLAYER_ID}\",\"content\":\"Hello! How are you today?\"}'"
echo

curl -s -X POST "${BASE_URL}/connections/${CONNECTION_ID}/messages" -H "Content-Type: application/json" -d "{\"player_id\":\"${PLAYER_ID}\",\"content\":\"Hello! How are you today?\"}"

echo
echo "Message sent! Now switch to the Player 2 window and continue the script there."
echo
read -p "After Player 2 has sent you a message, press enter to check for notifications..."

echo
echo "Step 4: Checking for notifications from Player 2..."
echo
echo "Running command:"
echo "curl -X GET ${BASE_URL}/players/${PLAYER_ID}/notifications"
echo

curl -s -X GET "${BASE_URL}/players/${PLAYER_ID}/notifications"

echo
read -p "Press enter to acknowledge notifications..."

echo
echo "Step 5: Acknowledging all notifications..."
echo
echo "Running command:"
echo "curl -X POST ${BASE_URL}/players/${PLAYER_ID}/notifications/ack"
echo

curl -s -X POST "${BASE_URL}/players/${PLAYER_ID}/notifications/ack"

echo
echo "Notifications acknowledged! Checking that they're gone..."
echo
echo "Running command:"
echo "curl -X GET ${BASE_URL}/players/${PLAYER_ID}/notifications"
echo

curl -s -X GET "${BASE_URL}/players/${PLAYER_ID}/notifications"

echo
echo "Testing complete for Player 1!"
echo