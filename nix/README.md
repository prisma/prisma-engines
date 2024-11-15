This directory contains a nix shell that is convenient to streamline developement, however,
contributors must not require to depend on nix for any specific workflow.

Instead, automation should be provided in a combination of bash scripts and docker, exposed over
tasks in the [root's Makefile](/Makefile)
