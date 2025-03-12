@echo off
setlocal EnableDelayedExpansion

set SERVER_IP=172.25.223.120
set SERVER_PORT=8080
set BASE_URL=http://%SERVER_IP%:%SERVER_PORT%

:: Generate a random GUID for the player
for /f "tokens=*" %%a in ('powershell -Command "[guid]::NewGuid().ToString('N')"') do set PLAYER_ID=%%a

echo ===== Friends Connect - Player 1 Script =====
echo.
echo This script will walk you through the Player 1 flow for testing
echo the friends-connect application.
echo.
echo Generated player ID: %PLAYER_ID%
echo.
echo Press any key to start...
pause > nul

echo.
echo Step 1: Creating a new connection for Player 1...
echo.
echo Running command:
echo curl -X POST %BASE_URL%/connections -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\"}"
echo.

for /f "tokens=* usebackq" %%a in (
  `curl -s -X POST %BASE_URL%/connections -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\"}"`
) do set RESP=%%a

echo Response: %RESP%

for /f "tokens=2 delims=:," %%a in ('echo %RESP% ^| findstr /C:"\"id\":"') do (
  set CONNECTION_ID=%%a
  set CONNECTION_ID=!CONNECTION_ID:"=!
  set CONNECTION_ID=!CONNECTION_ID: =!
)

for /f "tokens=2 delims=:," %%a in ('echo %RESP% ^| findstr /C:"\"link_id\":"') do (
  set LINK_ID=%%a
  set LINK_ID=!LINK_ID:"=!
  set LINK_ID=!LINK_ID: =!
)

echo.
echo Connection Created!
echo Connection ID: %CONNECTION_ID%
echo.
echo LINK TO SHARE: %BASE_URL%/connections/link/%LINK_ID%/join
echo.
echo Now in your other command prompt, run the Player 2 script with this link_id:
echo %LINK_ID%
echo.
echo After running the Player 2 script to the first pause, press any key to continue...
pause > nul

echo.
echo Step 2: Checking for notifications (Player 2 should have joined)...
echo.
echo Running command:
echo curl -X GET %BASE_URL%/players/%PLAYER_ID%/notifications
echo.

curl -s -X GET %BASE_URL%/players/%PLAYER_ID%/notifications

echo.
echo Press any key to continue...
pause > nul

echo.
echo Step 3: Sending a message to Player 2...
echo.
echo Running command:
echo curl -X POST %BASE_URL%/connections/%CONNECTION_ID%/messages -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\",\"content\":\"Hello! How are you today?\"}"
echo.

curl -s -X POST %BASE_URL%/connections/%CONNECTION_ID%/messages -H "Content-Type: application/json" -d "{\"player_id\":\"%PLAYER_ID%\",\"content\":\"Hello! How are you today?\"}"

echo.
echo Message sent! Now switch to the Player 2 window and continue the script there.
echo.
echo After Player 2 has sent you a message, press any key to check for notifications...
pause > nul

echo.
echo Step 4: Checking for notifications from Player 2...
echo.
echo Running command:
echo curl -X GET %BASE_URL%/players/%PLAYER_ID%/notifications
echo.

curl -s -X GET %BASE_URL%/players/%PLAYER_ID%/notifications

echo.
echo Press any key to acknowledge notifications...
pause > nul

echo.
echo Step 5: Acknowledging all notifications...
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
echo Testing complete for Player 1!
echo.