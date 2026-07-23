# razer-battery-report

Показывает уровень заряда беспроводных устройств Razer в системном трее Windows.
Иконка в трее отображает точный процент заряда числом.

## Поддерживаемые устройства

- Razer DeathAdder V3 Pro (Wired / Wireless)
- Razer DeathAdder V3 HyperSpeed (Wired / Wireless)
- Razer DeathAdder V2 Pro (Wired / Wireless)
- Razer Viper V2 Pro (Wired / Wireless)

## Пример иконки

Иконки — это SVG без шрифтов. В исходных файлах цвет задан токеном `__FG__`,
который во время работы заменяется на цвет, зависящий от уровня заряда.

<p>
  <img src="img/example-67.svg" width="96" height="96" alt="Иконка заряда 67 %">
</p>

## Запуск

Нужны установленные [Rust](https://www.rust-lang.org/) и [Git](https://git-scm.com/).

```sh
git clone https://github.com/xzeldon/razer-battery-report.git
cd razer-battery-report
cargo build --release
./target/release/razer-battery-report.exe
```

Готовый исполняемый файл: `target/release/razer-battery-report.exe`.

> Работает только на **Windows**.
