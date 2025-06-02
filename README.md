# pico-console
###
A retro-like games console running on Raspberry Pi Pico 2W.
<br>
All featured games support multiplayer for 2 players.
#

### **Games and features**
- **Snake**
    - snake body turns into apples upon death
- **Space Invaders**
    - boss level every 5 levels
    - only orange and red enemies can shoot
- **Sokoban**
    - 11 tricky levels
    - a menu made for easily restarting the level
- **Breakout**
    - smooth gameplay allowing for 50 active projectiles at a time
    - precise collision detection
    - blue powerups that spawn 3 balls when collected
    - orange powerups that spawn 2 more balls next to a random ball
 

### **Hardware requirements**
- Raspberry Pi Pico 2W with the [debugprobe](https://github.com/raspberrypi/debugprobe/releases/tag/debugprobe-v2.2.2) on it
- Another Raspberry Pi Pico 2W
- ST7735 128x160 screen

### Installing on the RP Pico 2W
This console relies on a [this webserver](https://github.com/addrian-77/rust-webserver) running on a separate device. 
<br>
It works by connecting to the hotspot of the device (change **WIFI_SSID.txt** and **WIFI_PASSWORD.txt** accordingly!).
- #### 1. Installing the webserver
  ```
  git clone https://github.com/addrian-77/rust-webserver
  cd ./rust-webserver
  cargo run
  ```
  The webserver is now running on localhost:7878 for player 1 and localhost:7879 for player 2.
- #### 2. The wiring configuration
  Make sure your components are wired correctly. If so, you can go to the next step.
  ![wiring_diagram-c4f1ad60a075d0dd17c390678627b1a1](https://github.com/user-attachments/assets/fea0777e-5f57-4f19-85fb-2254e71713cc)

- #### 3. Flashing the RP Pico 2W
  ```
  git clone https://github.com/addrian-77/rust-pico-console
  cd ./rust-pico-console
  cargo run -r
  ```
The console should now be running! Now head to your webserver controller and start playing!
