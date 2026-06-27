# Deploy Standalone Nginx via System-Manager — Dev Skill

## What We Learned

Deploying a simple nginx static file location via system-manager took 6
deployment attempts across 2 flake configs. Here's what we learned.

### Problem

The DASL notebooklm directory at `/var/www/solana.solfunmeme.com/notebooklm/`
needed HTTPS access. The existing nginx was managed by the pastebin flake
(`kant-pastebin-only`), which was slow to build (3+ minutes) and had
unrelated dependencies (kant pastebin, nora, etc.).

Adding a single `location /notebooklm/` block should be trivial, but the
system-manager + nix module system created multiple subtle failure modes.

### Failure Modes (in order)

1. **Wrong deployment target**: The `dasl-planning#all-services` config has
   `services.nginx.virtualHosts` but NOT `services.nginx.enable`. Adding
   locations there doesn't rebuild the nginx config. Nix eval confirms the
   config is correct, but system-manager never activates it.

2. **Dual nginx ownership**: Two system-manager flake deployments both
   trying to manage nginx. The pastebin flake owned `services.nginx.enable`,
   blocking the notebooklm flake from taking over.

3. **Conflicting SSL options**: `forceSSL` + `onlySSL` are mutually exclusive
   in nixpkgs' nginx module. `forceSSL` already implies SSL-only with redirect.

4. **Wrong SSL key path**: `privkey.key` vs `privkey.pem` — certbot creates
   `.pem` files, not `.key`.

5. **tmpfiles.d activation error**: System-manager's activation fails on
   tmpfiles when deploying alongside an existing system-manager deployment.
   This is a known bug (tmpfiles dir already exists). The workaround is to
   `systemctl stop nginx` before deploying the new flake.

6. **Permission denied on intermediate directories**: `/var/www/.../notebooklm/2026/06/`
   was owned by `root:root`, preventing nginx (running as `nginx` user) from
   traversing the path. Fix: `chown www-data:www-data` on all intermediate dirs.

### Working Solution

```nix
# ~/dasl-planning/notebooklm-deploy/flake.nix
{
  services.nginx = {
    enable = true;  # MUST be true — without this, locations are inert
    virtualHosts."solana.solfunmeme.com" = {
      serverName = "solana.solfunmeme.com";
      forceSSL = true;
      sslCertificate = "/etc/letsencrypt/live/solana.solfunmeme.com/fullchain.pem";
      sslCertificateKey = "/etc/letsencrypt/live/solana.solfunmeme.com/privkey.pem";
      # onlySSL is mutually exclusive with forceSSL — omit it

      locations."/notebooklm/" = {
        alias = "/var/www/solana.solfunmeme.com/notebooklm/";
        extraConfig = ''
          autoindex on;
          autoindex_exact_size off;
          autoindex_localtime on;
          add_header Cache-Control "no-store";
        '';
      };
    };
  };
}
```

### Deployment Steps

```bash
# 1. Stop old nginx (if managed by another flake)
sudo systemctl stop nginx

# 2. Deploy standalone flake
cd ~/dasl-planning/notebooklm-deploy
sudo system-manager switch --flake .#notebooklm

# 3. Verify
curl -sI https://solana.solfunmeme.com/notebooklm/ | head -5
# → HTTP/2 200
```

### Key Takeaways

- `services.nginx.enable = true` is REQUIRED for nginx config to be built
- `services.nginx.virtualHosts` without `enable` is silently ignored
- System-manager flake deployments don't merge — each is independent
- To replace an existing nginx deployment, stop it first, then deploy the new one
- Always verify SSL key paths: `ls /etc/letsencrypt/live/<domain>/privkey*`
- Directory ownership matters: nginx user must traverse all parent directories

### Git Repo

```
git+file:///mnt/data1/git/solana.solfunmeme.com/notebooklm-deploy.git
```
