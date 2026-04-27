{
  description = "Shashin (写真) — screenshot tool for macOS and Linux";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    crate2nix.url = "github:nix-community/crate2nix";
    flake-utils.url = "github:numtide/flake-utils";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    crate2nix,
    flake-utils,
    substrate,
  }:
    (import "${substrate}/lib/rust-tool-release-flake.nix" {
      inherit nixpkgs crate2nix flake-utils;
    }) {
      toolName = "shashin";
      src = self;
      repo = "pleme-io/shashin";

      # Migration to substrate module-trio + shikumiTypedGroups.
      # See kekkai (template) and hikki (enum + custom-gated daemon).
      # shashin demonstrates: bare-binary daemon (no subcommand) +
      # custom processType ("Interactive" for hotkey listener) wired
      # via extraHmConfigFn rather than withUserDaemon.
      module = {
        description = "Shashin (写真) — screenshot tool";
        hmNamespace = "blackmatter.components";

        # Shikumi YAML config at ~/.config/shashin/shashin.yaml.
        withShikumiConfig = true;

        shikumiTypedGroups = {
          capture = {
            default_mode   = {
              type        = nixpkgs.lib.types.enum [ "region" "window" "fullscreen" ];
              default     = "region";
              description = "Default capture mode.";
            };
            delay_ms       = { type = "int";  default = 0;     description = "Delay in milliseconds before capture."; };
            include_cursor = { type = "bool"; default = false; description = "Whether to include the cursor in captures."; };
          };

          output = {
            save_dir          = { type = "str"; default = "~/Pictures/Screenshots";  description = "Directory to save screenshots."; };
            format            = {
              type        = nixpkgs.lib.types.enum [ "png" "jpg" "webp" ];
              default     = "png";
              description = "Image format for saved screenshots.";
            };
            quality           = { type = "int"; default = 95;                                description = "Image quality (1-100, applicable to jpg/webp)."; };
            filename_template = { type = "str"; default = "shashin_%Y-%m-%d_%H-%M-%S";       description = "Filename template (supports strftime-style placeholders)."; };
          };

          annotation = {
            enabled       = { type = "bool";  default = true;     description = "Enable annotation mode after capture."; };
            default_color = { type = "str";   default = "#ff0000"; description = "Default annotation color (hex string)."; };
            line_width    = { type = "float"; default = 2.0;       description = "Line width in pixels for annotations."; };
            font_size     = { type = "float"; default = 16.0;      description = "Font size for text annotations."; };
          };

          hotkeys = {
            fullscreen = { type = "str"; default = "cmd+shift+3"; description = "Hotkey for fullscreen capture."; };
            region     = { type = "str"; default = "cmd+shift+4"; description = "Hotkey for region capture."; };
            window     = { type = "str"; default = "cmd+shift+5"; description = "Hotkey for window capture."; };
          };

          clipboard = {
            auto_copy       = { type = "bool";      default = true; description = "Automatically copy screenshot to clipboard."; };
            auto_clear_secs = { type = "nullOrInt"; default = null; description = "Seconds after which to clear clipboard (null = never)."; };
          };
        };

        extraHmOptions = {
          extraSettings = nixpkgs.lib.mkOption {
            type = nixpkgs.lib.types.attrs;
            default = { };
            description = "Additional raw settings merged on top of the typed YAML.";
          };
        };

        # Custom daemon wiring — bare binary (no subcommand) + Interactive
        # processType for the hotkey listener. Skips withUserDaemon since
        # the trio assumes a subcommand and Adaptive priority.
        extraHmConfigFn =
          { cfg, pkgs, lib, config, ... }:
          let
            hmHelpers = import "${substrate}/lib/hm/service-helpers.nix" {
              inherit lib;
            };
            isDarwin = pkgs.stdenv.hostPlatform.isDarwin;
            logDir =
              if isDarwin then "${config.home.homeDirectory}/Library/Logs"
              else "${config.home.homeDirectory}/.local/share/shashin/logs";
            extras = cfg.extraSettings;
          in lib.mkMerge [
            (lib.mkIf (extras != { }) {
              services.shashin.settings = extras;
            })

            {
              home.activation.shashin-log-dir =
                lib.hm.dag.entryAfter [ "writeBoundary" ] ''
                  run mkdir -p "${logDir}"
                '';
            }

            # Daemon runs unconditionally on cfg.enable. Trio already
            # gates extraHmConfigFn on cfg.enable, so no separate guard
            # needed.
            (lib.mkIf isDarwin
              (hmHelpers.mkLaunchdService {
                name = "shashin";
                label = "io.pleme.shashin";
                command = "${cfg.package}/bin/shashin";
                args = [ ];
                logDir = logDir;
                processType = "Interactive";
                keepAlive = true;
              }))

            (lib.mkIf (!isDarwin)
              (hmHelpers.mkSystemdService {
                name = "shashin";
                description = "Shashin — screenshot tool daemon";
                command = "${cfg.package}/bin/shashin";
                args = [ ];
              }))
          ];
      };
    };
}
