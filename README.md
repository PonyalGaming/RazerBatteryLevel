# razer-battery-report

Показывает уровень заряда беспроводных устройств Razer в системном трее Windows.
Иконка в трее отображает точный процент заряда числом.

## Поддерживаемые устройства

- Razer DeathAdder V3 Pro (Wired / Wireless)
- Razer DeathAdder V3 HyperSpeed (Wired / Wireless)
- Razer DeathAdder V2 Pro (Wired / Wireless)
- Razer Viper V2 Pro (Wired / Wireless)

## Пример иконки (заряд 67 %)

Иконки — это SVG без шрифтов. В исходных файлах цвет задан токеном `__FG__`,
который во время работы заменяется на цвет, зависящий от уровня заряда.

<p>
  <img src="img/example-67.svg" width="96" height="96" alt="Иконка заряда 67 %">
</p>

Исходник для значения `67` (`assets/icons/067.svg`):

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32">
  <rect x="2.85" y="5.00" width="10.80" height="4.00" rx="1.00" fill="__FG__"/>
  <rect x="10.25" y="16.60" width="4.00" height="8.60" rx="1.00" fill="__FG__"/>
  <rect x="2.85" y="23.00" width="10.80" height="4.00" rx="1.00" fill="__FG__"/>
  <rect x="2.25" y="16.60" width="4.00" height="8.60" rx="1.00" fill="__FG__"/>
  <rect x="2.25" y="5.60" width="4.00" height="8.60" rx="1.00" fill="__FG__"/>
  <rect x="2.85" y="14.00" width="10.80" height="4.00" rx="1.00" fill="__FG__"/>
  <rect x="18.35" y="5.00" width="10.80" height="4.00" rx="1.00" fill="__FG__"/>
  <rect x="25.75" y="5.60" width="4.00" height="8.60" rx="1.00" fill="__FG__"/>
  <rect x="25.75" y="16.60" width="4.00" height="8.60" rx="1.00" fill="__FG__"/>
</svg>
```

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
