{
  description = "Floating Dictionary Linux";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # rust
            rustc
            cargo
            rustfmt
            clippy
            rustPlatform.rustcSrc

            pkg-config
            openssl
            clang
            libclang.lib
            wayland
            libxkbcommon
            tesseract
            leptonica
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.vulkan-loader
            pkgs.libGL
          ];
          RUST_BACKTRACE = "1";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.libclang.lib}/lib/clang/${pkgs.libclang.version}/include";

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          shellHook = ''
            export PATH="$CARGO_HOME/bin:$PATH"
            export CARGO_HOME="$PWD/.cargo"
            mkdir -p .cargo
            echo '*' > .cargo/.gitignore
          '';
        };

        # Package build
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "floating-dictionary-linux";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
            makeWrapper # ✨ เพิ่มตรงนี้
          ];

          buildInputs = with pkgs; [
            wayland
            libxkbcommon
            tesseract
            leptonica
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            vulkan-loader
            libGL
            openssl
          ];

          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.libclang.lib}/lib/clang/${pkgs.libclang.version}/include";

          # ✨ เพิ่มส่วนนี้ - wrap binary ให้หา libraries
          postInstall = ''
            wrapProgram $out/bin/floating-dictionary-linux \
              --prefix LD_LIBRARY_PATH : ${
                pkgs.lib.makeLibraryPath [
                  pkgs.wayland
                  pkgs.libxkbcommon
                  pkgs.vulkan-loader
                  pkgs.libGL
                ]
              }
          '';
        };
      }
    );
}
