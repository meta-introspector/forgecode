
#  512  curl -L https://nixos.org/nix/install >install_nix.sh
#  curl ./install_nix.sh --daemon
#  514  bash ./install_nix.sh --daemon

sudo mv /etc/zshrc.backup-before-nix /etc/zshrc
sudo mv /etc/bash.bashrc.backup-before-nix /etc/bash.bashrc
sudo mv /etc/zsh/zshrc.backup-before-nix /etc/zsh/zshrc
sudo rm /etc/systemd/system/nix-daemon.service
sudo rm  /etc/profile.d/nix.sh /etc/profile.d/nix.sh.backup-before-nix
bash ./install_nix.sh --daemon --yes 

