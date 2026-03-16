{
  description = "terraform-forge-cli — CLI tool for generating Terraform providers from OpenAPI specs";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      substrate,
      ...
    }:
    let
      system = "aarch64-darwin";
      pkgs = import nixpkgs { inherit system; };

      props = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = props.package.version;
      pname = "terraform-forge-cli";

      package = pkgs.rustPlatform.buildRustPackage {
        inherit pname version;
        src = pkgs.lib.cleanSource ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = { };
        };
        doCheck = true;
        meta = {
          description = props.package.description;
          homepage = props.package.homepage;
          license = pkgs.lib.licenses.mit;
          mainProgram = "terraform-forge";
        };
      };
    in
    {
      packages.${system} = {
        terraform-forge-cli = package;
        default = package;
      };

      overlays.default = final: prev: {
        terraform-forge-cli = self.packages.${final.system}.default;
      };

      devShells.${system}.default = pkgs.mkShellNoCC {
        packages = [
          pkgs.rustc
          pkgs.cargo
          pkgs.rust-analyzer
          pkgs.clippy
          pkgs.rustfmt
        ];
      };

      formatter.${system} = pkgs.nixfmt-tree;
    };
}
