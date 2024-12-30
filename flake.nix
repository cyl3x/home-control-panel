{
  description = "home-control-panel";

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
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, crane, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      commonArgs = {
        src = pkgs.lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type:
            (pkgs.lib.hasSuffix "\.ttf" path) || (craneLib.filterCargoSources path type);
        };

        strictDeps = true;

        nativeBuildInputs = with pkgs; [
          autoPatchelfHook
          wrapGAppsHook
          pkg-config
        ];

        buildInputs = with pkgs; [
          libglvnd
          openssl
        ] ++ commonArgs.runtimeDependencies;

        runtimeDependencies = with pkgs; [
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

        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath commonArgs.buildInputs}";
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      home-control-panel = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });
    in {
      checks = { inherit home-control-panel; };
      packages = {
        inherit home-control-panel;
        default = home-control-panel;
      };

      devShells.default = craneLib.devShell {
        inputsFrom = [ home-control-panel ];

        packages = [ pkgs.rust-analyzer rustToolchain ];

        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath commonArgs.buildInputs}";
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        RUST_BACKTRACE = 1;
      };
    });
}
