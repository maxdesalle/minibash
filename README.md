# minibash

A lightweight shell inspired by bash (version 3.2). **minibash** supports all of the commands a traditional shell supports `env, cd, echo...` as well as operators `&&, |, >, >>, ;`

<p align="center">
<img src="https://media.giphy.com/media/P6qPAAW7jX5guAHcws/giphy.gif" width="65%" />
</p>

> **Note**: redirections to the left `< and <<` are currently not supported.

## Using it
To start the shell, execute the following command:
```bash
cargo run
```

Once in the shell, press `ctrl-d` or execute the `exit` command to leave.

## Acknowledgements

- James Elford's [Working with signals in Rust](https://www.jameselford.com/blog/working-with-signals-in-rust-pt1-whats-a-signal/)
- Josh Mcguigan's [Build Your Own Shell using Rust](https://www.joshmcguigan.com/blog/build-your-own-shell-rust/)

## License

This repository is released under the [MIT License](https://github.com/maxdesalle/minibash/blob/main/LICENSE).
