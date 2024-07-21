## Pixi2ces

Experimental tool to convert [pixi](https://pixi.sh/) lock files to a [Conda explicit spec (ces)](https://conda.io/projects/conda/en/latest/user-guide/tasks/manage-environments.html#building-identical-conda-environments). 
This tool borrows inspiration and code from [pixi-pack](https://github.com/Quantco/pixi-pack). Its purpose is to allow users to manage environments and locking using pixi, but then render a lock file 
that can be used directly by conda or mamba. The explicit spec only contains download urls so no additional environment solve is required by conda/mamba. 

## Installation

```bash
cargo install --locked --git https://github.com/synapticarbors/pixi2ces
```

## Usage

After using pixi to create a `pixi.lock` file, you can then render a platform and env specific conda-compatible explicit spec by running:

```bash
$ pixi2ces pixi.toml -p <PLATFORM> -e <ENV>
```

This will create a file `conda-<PLATFORM>-<ENV>.lock` (e.g `conda-linux-64-default.lock`), which can then be used to create a conda environment:

```bash
$ conda create -n my-env --file conda-linux-64-default.lock
```
