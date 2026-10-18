{
  description = "Linux battery-aware CPU power management daemon";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default =
      let pkgs = nixpkgs.legacyPackages.x86_64-linux;
      in pkgs.rustPlatform.buildRustPackage {
        pname = "battop";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        description = "Linux battery-aware CPU power management daemon";
        meta.mainProgram = "battop";
      };

    nixosModules.default = { config, pkgs, lib, ... }:
      let
        pkg = self.packages.x86_64-linux.default;
        toml = pkgs.formats.toml { };

        profileType = lib.types.submodule {
          options = {
            governor = lib.mkOption {
              type = lib.types.str;
              default = "powersave";
            };
            turbo = lib.mkOption {
              type = lib.types.enum [ "never" "auto" ];
              default = "never";
            };
          };
        };
      in
      {
        options.services.battop = {
          enable = lib.mkEnableOption "Battop battery-aware CPU power management daemon";

          battery = lib.mkOption {
            type = profileType;
            default = { };
          };

          charger = lib.mkOption {
            type = profileType;
            default = {
              governor = "performance";
              turbo = "auto";
            };
          };

          charge_control = lib.mkOption {
            type = lib.types.submodule {
              options = {
                stop_threshold = lib.mkOption {
                  type = lib.types.nullOr lib.types.int;
                  default = null;
                };
                start_threshold = lib.mkOption {
                  type = lib.types.nullOr lib.types.int;
                  default = null;
                };
                behaviour = lib.mkOption {
                  type = lib.types.nullOr (lib.types.enum [ "auto" "inhibit-charge" "force-discharge" ]);
                  default = null;
                };
              };
            };
            default = { };
          };

          poll_interval_secs = lib.mkOption {
            type = lib.types.int;
            default = 5;
          };

          log_level = lib.mkOption {
            type = lib.types.str;
            default = "info";
          };
        };

        config = lib.mkIf config.services.battop.enable {
          environment.systemPackages = [ pkg ];

          environment.etc."battop/battop.toml".source =
            toml.generate "battop.toml" {
              battery = config.services.battop.battery;
              charger = config.services.battop.charger;
              charge_control = config.services.battop.charge_control;
              general = {
                poll_interval_secs = config.services.battop.poll_interval_secs;
                log_level = config.services.battop.log_level;
              };
            };

          systemd.services.battop = {
            description = "Battop power management daemon";
            wantedBy = [ "multi-user.target" ];
            after = [ "multi-user.target" ];

            serviceConfig = {
              Type = "notify";
              ExecStart = "${pkg}/bin/battop -c /etc/battop/battop.toml";
              Restart = "on-failure";
              RestartSec = "5s";
              WatchdogSec = "30s";
              ProtectKernelTunables = false;
              NotifyAccess = "main";
              User = "root";
            };
          };
        };
      };
  };
}
