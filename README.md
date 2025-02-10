# Simple Shell

Steps to configure, build, run, and test the project.

## Building

```bash
make
```

## Testing

```bash
make check
```

## Clean

```bash
make clean
```

## Generate Documentation

Using the below command will generate the documentation and open it in your system's default browser.

```bash
make docs
```

Using the below command will generate the documentation under `target/doc` and will ***not*** open it automatically.

```bash
make docs-no-open
```

## Install Dependencies

In order to use git send-mail you need to run the following command:

```bash
make install-deps
```
