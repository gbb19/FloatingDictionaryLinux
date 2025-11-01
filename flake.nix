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
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy

            # Build tools (สำคัญ!)
            pkg-config
            clang
            libclang.lib

            # Wayland libraries
            wayland
            libxkbcommon

            # Tesseract OCR
            tesseract
            leptonica

            # X11 fallback (optional)
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
          ];

          # Critical: Set library paths
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.vulkan-loader
            pkgs.libGL
          ];

          # Environment variables
          RUST_BACKTRACE = "1";

          # สำคัญ: บอก bindgen ว่า libclang อยู่ที่ไหน
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          # สำหรับ clang headers
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.libclang.lib}/lib/clang/${pkgs.libclang.version}/include";
        };
      }
    );
}
