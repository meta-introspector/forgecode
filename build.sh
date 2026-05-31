systemd-run --user --scope --slice=nix-build-protected.slice \
  -p MemoryLow=2G \
  -p MemoryMax=8G \
  -p CPUWeight=200 \
  -p TasksMax=1024 \
  -p IOWeight=100 \
  nix build 
