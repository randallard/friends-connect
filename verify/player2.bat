@echo off
setlocal EnableDelayedExpansion

set SERVER_IP=172.25.223.120
set SERVER_PORT=8080
set BASE_URL=http://%SERVER_IP%:%SERVER_PORT%

:: Generate a random GUID for the player
for /f "tokens=*" %%a in ('powershell -Command "[guid]::NewGuid().ToString('N')"') do set PLAYER_ID=%%a

echo ===== Friends Connect - Player 2 Script =====
echo.
echo This script will walk you through the Player 2 flow for testing
echo the friends-connect application.
echo.
echo Generated player ID: %PLAYER_ID%
echo.
echo You'll need the LINK_ID from Player 1's script.
echo.
set /p LINK_ID=Enter the LINK_ID from Player 1: 

echo.
echo Step 1: Joining connection using link ID: %LINK_ID%
echo.
echo Running command:
echo curl -X POST %BASE_URL%/connections/link/%LINK_ID%/join -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\"}"
echo.

for /f "tokens=* usebackq" %%a in (
  `curl -s -X POST %BASE_URL%/connections/link/%LINK_ID%/join -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\"}"`
) do set RESP=%%a

echo Response: %RESP%

for /f "tokens=2 delims=:," %%a in ('echo %RESP% ^| findstr /C:"\"id\":"') do (
  set CONNECTION_ID=%%a
  set CONNECTION_ID=!CONNECTION_ID:"=!
  set CONNECTION_ID=!CONNECTION_ID: =!
)

echo.
echo Connection joined!
echo Connection ID: %CONNECTION_ID%
echo.
echo Now go back to the Player 1 window and continue the script.
echo After Player 1 has sent a message, press any key to continue...
pause > nul

echo.
echo Step 2: Checking for messages from Player 1...
echo.
echo Running command:
echo curl -X GET %BASE_URL%/players/%PLAYER_ID%/notifications
echo.

curl -s -X GET %BASE_URL%/players/%PLAYER_ID%/notifications

echo.
echo Press any key to send a reply...
pause > nul

echo.
echo Step 3: Sending a message back to Player 1...
echo.
echo Running command:
echo curl -X POST %BASE_URL%/connections/%CONNECTION_ID%/messages -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\",\"content\":\"I'm doing great, thanks for asking! How about you?\"}"
echo.

curl -s -X POST %BASE_URL%/connections/%CONNECTION_ID%/messages -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\",\"content\":\"I'm doing great, thanks for asking! How about you?\"}"

echo.
echo Message sent! Now switch back to Player 1 window and continue the script there.
echo.
echo Press any key to acknowledge notifications...
pause > nul

echo.
echo Step 4: Acknowledging all notifications...
echo.
echo Running command:
echo curl -X POST %BASE_URL%/players/%PLAYER_ID%/notifications/ack
echo.

curl -s -X POST %BASE_URL%/players/%PLAYER_ID%/notifications/ack

echo.
echo Notifications acknowledged! Checking that they're gone...
echo.
echo Running command:
echo curl -X GET %BASE_URL%/players/%PLAYER_ID%/notifications
echo.

curl -s -X GET %BASE_URL%/players/%PLAYER_ID%/notifications

echo.
echo Testing complete for Player 2!
echo.