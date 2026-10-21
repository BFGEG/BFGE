# BFGE
Blazingly Fast Graphic Engine


Пример загрузки и вращения модели через публичный API:

```bash
cargo run --example rotating_model
cargo run --example rotating_model -- path/to/model.gltf
```

Закрытие окна завершает цикл и освобождает движок. Пример требует графической
сессии; обычные тесты цикла выполняются без окна и GPU.
