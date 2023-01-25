{ pkgs, flakeInputs, system, self', ... }:

# Run qemu with a disk image containing prisma-engines repo, docker and all the
# packages to build and test engines.
#
# This is useful for testing engines with e.g. artificial memory or cpu limits.
#
# You can run it using:
#
# ```
# $ nix run .#dev-vm
# ```
#
# This will boot the VM and create a nixos.qcow2 VM image file, or reuse it if
# it is already there.
#
# You can pass extra arguments to the qemu command line, they will be forwarded
# (see --help for example). That lets you easily constrain the VM's resources
# (CPU, RAM, network, disk IO), among other things.
#
# The recommended way to interact with the vm is through SSH. It listens on
# 2222 on the host's localhost:
#
# ```
# $ ssh prisma@localhost -p 2222
# ```
#
# Both the username and password are "prisma".
#
# Links:
# - https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/virtualisation/qemu-vm.nix
let
  evalConfig = import "${flakeInputs.nixpkgs}/nixos/lib/eval-config.nix";
  prisma-engines = self'.packages.prisma-engines;
  prisma-engines-inputs = prisma-engines.buildInputs ++ prisma-engines.nativeBuildInputs;
  vmConfig = (evalConfig {
    modules = [
      {
        system.stateVersion = "23.05";
        virtualisation.docker.enable = true;

        virtualisation.vmVariant.virtualisation = {
          diskSize = 1024 * 8; # 8GB
          forwardPorts = [
            {
              from = "host";
              host.port = 2222;
              guest.port = 22;
            }
          ];
          writableStore = true;
          writableStoreUseTmpfs = false;
          sharedDirectories.prisma-engines = {
            source = "${prisma-engines.src}";
            target = "/home/prisma/prisma-engines";
          };
        };

        # Enable flakes in the host vm
        nix = {
          # package = pkgs.nixUnstable;
          extraOptions = "experimental-features = nix-command flakes";
        };

        environment.systemPackages = with pkgs; [
          tmux
          git
          coreutils
          gnumake
        ] ++ prisma-engines-inputs;

        services.openssh = {
          listenAddresses = [{
            addr = "0.0.0.0";
            port = 22;
          }];
          enable = true;
          passwordAuthentication = true;
        };

        users.users.prisma = {
          isNormalUser = true;
          extraGroups = [
            "docker"
            "wheel" # Enable ‘sudo’ for the user.
          ];
          password = "prisma";
        };

      }
    ];
    system = "x86_64-linux";
  }
  ).config;
in
{
  packages.dev-vm = vmConfig.system.build.vm;
}
