# Ergodox Live Training Driver

[![codecov](https://codecov.io/gh/dmweis/ergodox-live-training-driver/branch/main/graph/badge.svg)](https://codecov.io/gh/dmweis/ergodox-live-training-driver)
[![Rust](https://github.com/dmweis/ergodox-live-training-driver/workflows/Rust/badge.svg)](https://github.com/dmweis/ergodox-live-training-driver/actions)
[![Private docs](https://github.com/dmweis/ergodox-live-training-driver/workflows/Deploy%20Docs%20to%20GitHub%20Pages/badge.svg)](https://davidweis.dev/ergodox-live-training-driver/ergodox_driver/index.html)

Driver for the live training protocol for the GMK firmware used on the Erogdox-ez keyboard.

[Private docs](https://davidweis.dev/ergodox-live-training-driver/ergodox_live_training_driver/)

## Install open_ergodox_layout

```shell
cargo install --git https://github.com/dmweis/ergodox-live-training-driver --bin open_ergodox_layout
```

## System dependencies

On ubuntu you may need `libusb-1.0-0-dev` and `libudev-dev`.

## Training website

It seems that the ergodox training websites GraphQL backend has moved.
Used to be at `https://oryx.ergodox-ez.com/graphql` but now is at `https://oryx.zsa.io/graphql`.
If this changes in the future you can easily check by:

1. Open Oryx live trainer
2. Open developer tools and network tab
3. Connect your keyboard
4. Look for requests to some graphql endpoint

## License

This project is dual licensed under MIT and Apache licenses.

## Disclaimer

All product names, logos, and brands are property of their respective owners. All company, product and service names used in this website are for identification purposes only. Use of these names, logos, and brands does not imply endorsement.
