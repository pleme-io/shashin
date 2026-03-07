# Shashin home-manager module — screenshot tool with typed config + daemon
#
# Namespace: blackmatter.components.shashin.*
#
# Generates YAML config from typed Nix options, loaded by shikumi at runtime.
# Supports hot-reload via symlink-aware file watching.
#
# Module factory: receives { hmHelpers } from flake.nix, returns HM module.
{ hmHelpers }:
{
  lib,
  config,
  pkgs,
  ...
}:
with lib;
let
  inherit (hmHelpers) mkLaunchdService mkSystemdService;
  cfg = config.blackmatter.components.shashin;
  isDarwin = pkgs.stdenv.isDarwin;

  logDir =
    if isDarwin then "${config.home.homeDirectory}/Library/Logs"
    else "${config.home.homeDirectory}/.local/share/shashin/logs";

  # -- YAML config generation --------------------------------------------------
  settingsAttr = let
    capture = filterAttrs (_: v: v != null) {
      default_mode = cfg.capture.default_mode;
      delay_ms = cfg.capture.delay_ms;
      include_cursor = cfg.capture.include_cursor;
    };

    output = filterAttrs (_: v: v != null) {
      save_dir = cfg.output.save_dir;
      format = cfg.output.format;
      quality = cfg.output.quality;
      filename_template = cfg.output.filename_template;
    };

    annotation = filterAttrs (_: v: v != null) {
      enabled = cfg.annotation.enabled;
      default_color = cfg.annotation.default_color;
      line_width = cfg.annotation.line_width;
      font_size = cfg.annotation.font_size;
    };

    hotkeys = filterAttrs (_: v: v != null) {
      fullscreen = cfg.hotkeys.fullscreen;
      region = cfg.hotkeys.region;
      window = cfg.hotkeys.window;
    };

    clipboard = filterAttrs (_: v: v != null) {
      auto_copy = cfg.clipboard.auto_copy;
      auto_clear_secs = cfg.clipboard.auto_clear_secs;
    };
  in
    filterAttrs (_: v: v != {} && v != null) {
      inherit capture output annotation hotkeys clipboard;
    }
    // cfg.extraSettings;

  yamlConfig = pkgs.writeText "shashin.yaml"
    (lib.generators.toYAML { } settingsAttr);
in
{
  options.blackmatter.components.shashin = {
    enable = mkEnableOption "Shashin — screenshot tool";

    package = mkOption {
      type = types.package;
      default = pkgs.shashin;
      description = "The shashin package to use.";
    };

    # -- Capture ---------------------------------------------------------------
    capture = {
      default_mode = mkOption {
        type = types.enum [ "region" "window" "fullscreen" ];
        default = "region";
        description = "Default capture mode.";
      };

      delay_ms = mkOption {
        type = types.int;
        default = 0;
        description = "Delay in milliseconds before capture.";
      };

      include_cursor = mkOption {
        type = types.bool;
        default = false;
        description = "Whether to include the cursor in captures.";
      };
    };

    # -- Output ----------------------------------------------------------------
    output = {
      save_dir = mkOption {
        type = types.str;
        default = "~/Pictures/Screenshots";
        description = "Directory to save screenshots.";
      };

      format = mkOption {
        type = types.enum [ "png" "jpg" "webp" ];
        default = "png";
        description = "Image format for saved screenshots.";
      };

      quality = mkOption {
        type = types.int;
        default = 95;
        description = "Image quality (1-100, applicable to jpg/webp).";
      };

      filename_template = mkOption {
        type = types.str;
        default = "shashin_%Y-%m-%d_%H-%M-%S";
        description = "Filename template (supports strftime-style placeholders).";
      };
    };

    # -- Annotation ------------------------------------------------------------
    annotation = {
      enabled = mkOption {
        type = types.bool;
        default = true;
        description = "Enable annotation mode after capture.";
      };

      default_color = mkOption {
        type = types.str;
        default = "#ff0000";
        description = "Default annotation color (hex string).";
      };

      line_width = mkOption {
        type = types.float;
        default = 2.0;
        description = "Line width in pixels for annotations.";
      };

      font_size = mkOption {
        type = types.float;
        default = 16.0;
        description = "Font size for text annotations.";
      };
    };

    # -- Hotkeys ---------------------------------------------------------------
    hotkeys = {
      fullscreen = mkOption {
        type = types.str;
        default = "cmd+shift+3";
        description = "Hotkey for fullscreen capture.";
      };

      region = mkOption {
        type = types.str;
        default = "cmd+shift+4";
        description = "Hotkey for region capture.";
      };

      window = mkOption {
        type = types.str;
        default = "cmd+shift+5";
        description = "Hotkey for window capture.";
      };
    };

    # -- Clipboard -------------------------------------------------------------
    clipboard = {
      auto_copy = mkOption {
        type = types.bool;
        default = true;
        description = "Automatically copy screenshot to clipboard.";
      };

      auto_clear_secs = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Seconds after which to clear clipboard (null = never).";
      };
    };

    # -- Escape hatch ----------------------------------------------------------
    extraSettings = mkOption {
      type = types.attrs;
      default = {};
      description = ''
        Additional raw settings merged on top of typed options.
        Use this for experimental or newly-added config keys not yet
        covered by typed options. Values are serialized directly to YAML.
      '';
    };
  };

  config = mkIf cfg.enable (mkMerge [
    # Install the package
    {
      home.packages = [ cfg.package ];
    }

    # Create log directory
    {
      home.activation.shashin-log-dir = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        run mkdir -p "${logDir}"
      '';
    }

    # YAML configuration -- always generated from typed options
    {
      xdg.configFile."shashin/shashin.yaml".source = yamlConfig;
    }

    # Darwin: launchd agent (daemon mode for hotkey listener)
    (mkIf isDarwin
      (mkLaunchdService {
        name = "shashin";
        label = "io.pleme.shashin";
        command = "${cfg.package}/bin/shashin";
        args = [ ];
        logDir = logDir;
        processType = "Interactive";
        keepAlive = true;
      })
    )

    # Linux: systemd user service (hotkey listener daemon)
    (mkIf (!isDarwin)
      (mkSystemdService {
        name = "shashin";
        description = "Shashin — screenshot tool daemon";
        command = "${cfg.package}/bin/shashin";
        args = [ ];
      })
    )
  ]);
}
