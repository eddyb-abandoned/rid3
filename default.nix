let
   pkgs = import <nixpkgs> {};
 in rec {
   r3Env = pkgs.stdenv.mkDerivation rec {
     name = "r3-env";
     buildInputs = with pkgs; with pkgs.xlibs; [
        stdenv
        pkgconfig

        # glfw deps
        cmake mesa libXrandr libXi libXxf86vm libXfixes x11

        mesa_noglu

        libXcursor

        glib
        zlib
        expat
        openssl
     ];
   };
 }