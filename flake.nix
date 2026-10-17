{
  description = "Linux battery-aware CPU power management daemon";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
  };

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default =
      let
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
        rustPkgs = pkgs.rustPlatform.buildRustPackage {
          pname = "battop";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = {
            description = "Linux battery-aware CPU power management daemon";
            mainProgram = "battop";
          };
        };
      in
      rustPkgs;

    nixosModules.default = { pkgs, ... }: {
      environment.systemPackages = [ self.packages.x86_64-linux.default ];
      systemd.services.battop = {
        description = "Battop power management daemon";
        after = [ "multi-user.target" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${self.packages.x86_64-linux.default}/bin/battop";
          Restart = "on-failure";
          RestartSec = "5s";
        };
      };
    };
  };
}
