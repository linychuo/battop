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
            license = pkgs.lib.licenses.mit;
          };
        };
      in
      rustPkgs;

    nixosModules.default = { config, pkgs, lib, ... }:
      let
        cfg = config.services.battop;

        tomlFormat = pkgs.formats.toml { };

        generateConfig = tomlFormat.generate "battop.toml" {
          battery = {
            governor = cfg.battery.governor;
            turbo = cfg.battery.turbo;
          } // lib.optionalAttrs (cfg.battery.chargeStopThreshold != null) {
            charge_stop_threshold = cfg.battery.chargeStopThreshold;
          } // lib.optionalAttrs (cfg.battery.chargeStartThreshold != null) {
            charge_start_threshold = cfg.battery.chargeStartThreshold;
          } // lib.optionalAttrs (cfg.battery.chargeBehaviour != null) {
            charge_behaviour = cfg.battery.chargeBehaviour;
          };

          charger = {
            governor = cfg.charger.governor;
            turbo = cfg.charger.turbo;
          } // lib.optionalAttrs (cfg.charger.chargeStopThreshold != null) {
            charge_stop_threshold = cfg.charger.chargeStopThreshold;
          } // lib.optionalAttrs (cfg.charger.chargeStartThreshold != null) {
            charge_start_threshold = cfg.charger.chargeStartThreshold;
          } // lib.optionalAttrs (cfg.charger.chargeBehaviour != null) {
            charge_behaviour = cfg.charger.chargeBehaviour;
          };

          general = {
            poll_interval_secs = cfg.pollIntervalSecs;
            log_level = cfg.logLevel;
          };
        };

        battopPackage = self.packages.x86_64-linux.default;
      in
      {
        options.services.battop = {
          enable = lib.mkEnableOption "Battop battery-aware CPU power management daemon";

          package = lib.mkOption {
            type = lib.types.package;
            default = battopPackage;
            description = "The battop package to use.";
          };

          battery = {
            governor = lib.mkOption {
              type = lib.types.str;
              default = "powersave";
              description = "CPU governor when on battery.";
            };
            turbo = lib.mkOption {
              type = lib.types.str;
              default = "never";
              description = "Turbo boost mode when on battery (never or auto).";
            };
            chargeStopThreshold = lib.mkOption {
              type = lib.types.nullOr (lib.types.addCheck lib.types.int (v: v >= 0 && v <= 100));
              default = 80;
              description = "Stop charging at this percentage (0-100, null to disable).";
            };
            chargeStartThreshold = lib.mkOption {
              type = lib.types.nullOr (lib.types.addCheck lib.types.int (v: v >= 0 && v <= 100));
              default = 40;
              description = "Start charging at this percentage (0-100, null to disable).";
            };
            chargeBehaviour = lib.mkOption {
              type = lib.types.nullOr (lib.types.enum [ "auto" "inhibit-charge" "force-discharge" ]);
              default = null;
              description = "Charge behaviour (null to use hardware default).";
            };
          };

          charger = {
            governor = lib.mkOption {
              type = lib.types.str;
              default = "performance";
              description = "CPU governor when on AC charger.";
            };
            turbo = lib.mkOption {
              type = lib.types.str;
              default = "auto";
              description = "Turbo boost mode when on charger (never or auto).";
            };
            chargeStopThreshold = lib.mkOption {
              type = lib.types.nullOr (lib.types.addCheck lib.types.int (v: v >= 0 && v <= 100));
              default = 100;
              description = "Stop charging at this percentage when on AC (0-100, null to disable).";
            };
            chargeStartThreshold = lib.mkOption {
              type = lib.types.nullOr (lib.types.addCheck lib.types.int (v: v >= 0 && v <= 100));
              default = 0;
              description = "Start charging at this percentage when on AC (0-100, null to disable).";
            };
            chargeBehaviour = lib.mkOption {
              type = lib.types.nullOr (lib.types.enum [ "auto" "inhibit-charge" "force-discharge" ]);
              default = null;
              description = "Charge behaviour on AC (null to use hardware default).";
            };
          };

          pollIntervalSecs = lib.mkOption {
            type = lib.types.addCheck lib.types.int (v: v > 0);
            default = 5;
            description = "How often to poll power supply status (seconds).";
          };

          logLevel = lib.mkOption {
            type = lib.types.str;
            default = "info";
            description = "Log level (error, warn, info, debug, trace).";
          };
        };

        config = lib.mkIf cfg.enable {
          environment.systemPackages = [ cfg.package ];

          environment.etc."battop/battop.toml".source = generateConfig;

          systemd.services.battop = {
            description = "Battop power management daemon";
            wantedBy = [ "multi-user.target" ];
            after = [ "multi-user.target" ];

            serviceConfig = {
              Type = "notify";
              ExecStart = "${cfg.package}/bin/battop";
              Restart = "on-failure";
              RestartSec = "5s";
              WatchdogSec = "30s";

              # Write access to sysfs for CPU governor and battery control
              ProtectSystem = "strict";
              PrivateTmp = true;
              ProtectHome = true;
              ProtectKernelTunables = false;
              ProtectControlGroups = true;
              RestrictNamespaces = true;
              MemoryDenyWriteExecute = true;
              LockPersonality = true;

              # sd_notify
              NotifyAccess = "main";

              # Need root for sysfs writes
              User = "root";
            };
          };
        };
      };
  };
}
