# FREAK Engine — публичный API

Документ описывает фактический клиентский API версии 0.1.0 с главным циклом из
[PR #45](https://github.com/BFGEG/BFGE/pull/45). Примеры подключения и использования —
в [руководстве пользователя](User_Guide.md), полный Rustdoc генерируется командой
`cargo doc --no-deps --all-features --locked` из корня репозитория.

## Точки входа и типы

`Engine::new()` и `Engine::default()` создают движок без окна/GPU. Методы фасада
реализованы трейтом `EngineApi`: клиенту нужен `use BFGE::{Engine, EngineApi}`.
Все основные типы ниже реэкспортированы из корня `BFGE`; математические типы
`Vec3`, `Quat` и `Mat4` — из `BFGE::math`. Внутренние модули пока также публичны,
но клиентские примеры используют фасад.

| Тип | Назначение |
| --- | --- |
| `WindowDescriptor` | `title: String`, `size: WindowSize`; конструктор `new(&str, u32, u32)`. |
| `WindowSize` | `width: u32`, `height: u32` в физических пикселях; `new(u32, u32)`. |
| `SceneNodeId`, `SceneObjectId` | Непрозрачные `Copy`-идентификаторы, получаемые от движка. Не проверяют принадлежность сессии; после reset могут совпасть с ID новых сущностей. |
| `Transform` | `translation: Vec3`, `rotation: Quat`, `scale: Vec3`. По умолчанию нулевой перенос, единичный quaternion и единичный масштаб. |
| `ViewpointState` | `position: Vec3`, `direction: Vec3`, `fov_y: f32` в радианах. По умолчанию `(0, 0, 5)`, `(0, 0, -1)`, 60°. |
| `EventTime` | Два интервала в секундах (`f32`); семантика описана ниже. |
| `EngineEvent` | Два обработчика `frame_started` / `frame_ended` типа `Box<dyn FnMut(&mut dyn SceneApi, &EventTime) -> Result<(), SceneError>>`. |
| `EngineError`, `SceneError`, `ModelLoadError`, `ScenePersistenceError` | Типизированные ошибки с `Display` и `std::error::Error`, отмечены `#[non_exhaustive]`. |

## Трейт EngineApi

Сигнатуры соответствуют `src/core/api.rs`:

```rust
use std::path::Path;

use BFGE::{
    EngineError, EngineEvent, ModelLoadError, SceneApi, SceneNodeId,
    SceneObjectId, ScenePersistenceError, WindowDescriptor, WindowSize,
};

pub trait EngineApi {
    fn initialize(&mut self, window: WindowDescriptor) -> Result<(), EngineError>;
    fn run(&mut self) -> Result<(), EngineError>;
    fn add_event(&mut self, event: EngineEvent) -> Result<(), EngineError>;
    fn shutdown(&mut self) -> Result<(), EngineError>;
    fn scene(&mut self) -> &mut dyn SceneApi;
    fn load_model(
        &mut self,
        path: &Path,
        parent: SceneNodeId,
    ) -> Result<SceneObjectId, ModelLoadError>;
    fn set_window_size(&mut self, size: WindowSize) -> Result<(), EngineError>;
    fn save_scene(&mut self) -> Result<(), ScenePersistenceError>;
    fn restore_scene(&mut self) -> Result<(), ScenePersistenceError>;
}
```

- `initialize` создаёт окно и графику на главном потоке, сохраняя сцену,
  подготовленную до него. Нулевые размеры отклоняются. Повторный вызов при активной
  сессии возвращает `AlreadyInitialized`; ошибки окна — `Platform`, графики —
  `Lifecycle`. Используйте один `Engine` между сессиями: оконный цикл создаётся
  один раз на процесс.
- `run` блокирует поток до закрытия окна или ошибки. Он не освобождает подсистемы:
  после возврата нужен `shutdown`. До инициализации/после shutdown — `NotInitialized`.
- `add_event` сохраняет обработчики в порядке регистрации; доступен до initialize,
  сейчас всегда успешен. Обработчики владеют захваченными данными; захваченные
  заимствования должны быть `'static`. Отмены регистрации нет.
- `shutdown` освобождает графику/окно и сбрасывает сцену и ресурсы. Снимок,
  обработчики и оконный цикл сохраняются. Без активной сессии — `NotInitialized`.
- `scene` доступен сразу после `Engine::new` и после shutdown. Новая сцена содержит
  корень и точку обзора, без объектов. ID старой сцены без проверки поколения
  могут адресовать новые сущности после сброса.
- `load_model` создаёт новый объект, привязанный к существующему `parent`; доступен
  без окна. Повторная загрузка создаёт новый объект. Ошибка не меняет сцену/ресурсы.
- `set_window_size` требует активной сессии и ненулевых размеров. Сначала проверяет
  и перенастраивает backend, затем запрашивает resize окна; ошибки —
  `NotInitialized`, `Platform` или `Render`. Во время `run` клиент не имеет доступа
  к движку, обработчики получают только сцену; native resize обрабатывается циклом.
- `save_scene` заменяет единственный снимок в памяти: узлы, объекты, меши и точка
  обзора. Доступен без окна и сейчас всегда успешен. Снимок переживает shutdown,
  теряется при уничтожении движка; окно и обработчики не сериализуются.
- `restore_scene` целиком заменяет сцену/ресурсы снимком, только если нет объектов
  (одни узлы допустимы). Проверка занятости предшествует проверке снимка:
  `SceneNotEmpty` возвращается даже без сохранения. В свободной сцене отсутствие
  снимка — успешный no-op. Невалидный снимок даёт `Restore` без частичных изменений.

## Трейт SceneApi

Сигнатуры соответствуют `src/scene/api.rs`:

```rust
use BFGE::{SceneError, SceneNodeId, SceneObjectId, Transform, ViewpointState};

pub trait SceneApi {
    fn root_node(&self) -> SceneNodeId;
    fn create_node(&mut self, parent: SceneNodeId) -> Result<SceneNodeId, SceneError>;
    fn set_node_transform(
        &mut self,
        node: SceneNodeId,
        transform: Transform,
    ) -> Result<(), SceneError>;
    fn reparent_object(
        &mut self,
        object: SceneObjectId,
        new_parent: SceneNodeId,
    ) -> Result<(), SceneError>;
    fn viewpoint(&self) -> ViewpointState;
    fn set_viewpoint(&mut self, state: ViewpointState) -> Result<(), SceneError>;

    /// Возвращает фоновый и направленный свет сцены.
    fn lighting(&self) -> SceneLighting;

    /// Задаёт освещение последующих кадров. Некорректные значения отвергаются.
    fn set_lighting(&mut self, lighting: SceneLighting) -> Result<(), SceneError>;
}
```

`SceneLighting` содержит `ambient: Vec3` и `directional: DirectionalLight`.
У направленного света есть `direction: Vec3` (направление хода лучей) и
`color: Vec3` (линейная RGB-интенсивность). Цвета должны быть конечными и
неотрицательными, направление — конечным и нормализуемым. Освещение сохраняется
вместе со сценой; нормали для него вычисляются из треугольников модели.

`create_node` создаёт дочерний узел с единичным преобразованием.
`set_node_transform` заменяет все компоненты относительно родителя; для корня —
относительно сцены. Мировая матрица — `parent_world * local`, локальная применяет
масштаб, поворот и перенос. Неизвестный узел возвращает `UnknownNode`.

`reparent_object` переносит объект к другому узлу без сохранения мировой позиции,
не перемещая поддерево. Ошибки `UnknownObject` и `UnknownNode` не меняют привязки.
Точка обзора независима от узлов. `set_viewpoint` сейчас всегда успешен;
численные значения камеры/преобразований сохраняются без валидации. Требования
к значениям и примеры — в [руководстве](User_Guide.md#точка-обзора).

## Главный цикл и EventTime

Перед кадром обрабатываются события окна; закрытие очищает очередь без
дополнительного кадра, resize применяется перед сборкой проекции. Затем
вызываются все `frame_started`, выполняется сборка/рендер и все `frame_ended`.
Изменения в `frame_started` видны в текущем кадре, в `frame_ended` — в следующем.
Backend может пропустить показ при timeout/перекрытии окна; успешный возврат
рендера всё равно вызывает `frame_ended`. Ошибка рендера пропускает эту фазу.
При нулевой ширине/высоте обработчики и рендер приостановлены, события принимаются.

Ошибка обработчика немедленно возвращается как `EngineError::Scene`; изменения
не откатываются. При любом возврате из `run` флаг работы и очередь сбрасываются,
обработчики/сцена сохраняются. Перед повторным запуском после shutdown нужно
обеспечить актуальность ID, захваченных прежними обработчиками.

`time_since_last_event` — секунды с предыдущего вызова любого обработчика,
для первого — с запуска цикла. `time_since_last_frame` — интервал между началами
кадров, одинаковый для всех обработчиков кадра; для первого — с запуска цикла.
Время минимизации включается в первый интервал после восстановления.

## Загрузка моделей и ошибки

Поддерживаются glTF 2.0 (`.gltf`, `.glb`) и OBJ (`.obj`) без учёта регистра
расширения. Загружается статическая треугольная геометрия; все части составной
модели остаются одним объектом. Выбирается первый явно заданный цвет используемого
материала: `baseColorFactor` или MTL `Kd`/`d`, иначе linear RGBA `[0.8, 0.8, 0.8, 1.0]`.
Текстуры, освещение, анимационные каналы и alpha blending не реализованы.

Ограничения glTF/OBJ и буферов перечислены в
[руководстве](User_Guide.md#загрузка-моделей) и
[архитектуре, §18.2](FREAK_Engine_Architecture.md#182-импорт-моделей).
Неизвестный родитель проверяется до I/O и возвращает
`ModelLoadError::Scene(SceneError::UnknownNode(...))`. Ошибки чтения содержат
путь модели/буфера/MTL и причину (`Io { path, source }`), ошибки разбора —
путь и сообщение (`Parse { path, message }`). Неизвестный формат даёт
`UnsupportedFormat`. Для внешнего сопоставления типов ошибок нужна ветка `_`.

## Операции вне текущего клиентского API

Нет операций перепривязки узлов, удаления/скрытия объектов, преобразования
объекта отдельно от узла, загрузки текстур, изменения геометрии/цвета модели,
создания объекта из встроенного меша, доставки клавиатуры/мыши или произвольных
пользовательских систем. Нет пошагового update/render, offscreen-вывода,
остановки из обработчика или сохранения сцены на диск. Целевая
[архитектура](FREAK_Engine_Architecture.md) может описывать будущие возможности.
