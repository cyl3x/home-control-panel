{
  description = "Flake home-control-panel";

  nixConfig = {
    extra-trusted-substituters = [
      "https://home-control-panel.cachix.org"
    ];
    extra-trusted-public-keys = [
      "home-control-panel.cachix.org-1:oFAMn0verQX4hIEJYrxpvVd8egU8M0szyC/7wy4eBYE="
    ];
  };

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-flake.url = "github:juspay/rust-flake";
    rust-flake.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.flake-parts.flakeModules.easyOverlay
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
      ];

      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        nixpkgs.crossSystem = "aarch64-linux";

        rust-project = {
          crates."home-control-panel".crane = rec {
            args.nativeBuildInputs = with pkgs; [
              wrapGAppsHook
              makeWrapper
              pkg-config
            ];

            args.buildInputs = with pkgs; [
              gst_all_1.gst-plugins-bad
              gst_all_1.gst-plugins-base
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-rs
              gst_all_1.gst-vaapi
              gst_all_1.gstreamer
              libxkbcommon
              vulkan-loader
              wayland
            ];

            args.CARGO_BUILD_RUSTFLAGS = "-C symbol-mangling-version=v0";

            extraBuildArgs = {
              runtimeDependenciesPath = pkgs.lib.makeLibraryPath args.buildInputs;

              preFixup = ''
                wrapProgram "$out/bin/home-control-panel" \
                  "''${gappsWrapperArgs[@]}" \
                  --prefix LD_LIBRARY_PATH : "$runtimeDependenciesPath"
              '';
            };
          };

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type:
              (pkgs.lib.hasSuffix "\.ttf" path) ||
              (config.rust-project.crane-lib.filterCargoSources path type)
            ;
          };
        };

        overlayAttrs = { inherit (self'.packages) home-control-panel; };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self'.devShells.rust ];

          RUST_LOG = "info";
          RUST_BACKTRACE = "full";
          LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${self'.packages.home-control-panel.runtimeDependenciesPath}";
        };
      };
    };
}
