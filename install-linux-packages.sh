# # search for packages
# sudo apt-cache search webkit2gtk-4.0

# could not find system library 'alsa' required by the 'alsa-sys' crate
sudo apt-get install -y libasound2-dev portaudio19-dev build-essential libpulse-dev libdbus-1-dev 
# could not find system library 'openssl' required by the 'openssl-sys' crate
sudo apt-get install pkg-config libssl-dev
# could not find system library 'libsoup-2.4' required by the 'soup2-sys' crate
sudo apt install libsoup2.4-dev
# could not find system library 'atk' required by the 'atk-sys' crate
sudo apt-get install libatk3.0-cil librust-atk-dev
# could not find system library 'cairo' required by the 'cairo-sys-rs' crate
sudo apt-get install libcairo2-dev
# install gtk on linux
sudo apt-get install libgtk-3-dev libjavascriptcoregtk-4.0-dev libwebkit2gtk-4.0-dev
# build to verify that everything works
cargo build