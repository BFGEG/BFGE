# FREAK Engine — архитектурная спецификация

## 1. Назначение документа

Этот документ фиксирует целевую архитектуру графического движка **FREAK Engine** на Rust.

Архитектура ориентируется прежде всего на идеи OGRE:

- `Root` как оркестратор;
- `SceneManager` как владелец сцены;
- `SceneNode` как пространственная иерархия;
- `MovableObject` как содержимое узла;
- `Entity` как экземпляр отображаемого объекта;
- `Mesh`, `Material`, `Texture` как разделяемые ресурсы;
- `RenderSystem` как абстракция конкретного графического API.

При этом архитектура не должна буквально копировать C++-модель OGRE. Она должна быть адаптирована к Rust и его типовой модели владения, ссылок, enum-типов и trait-интерфейсов.

---

# 2. Основные архитектурные принципы

1. Сцена должна быть отделена от рендеринга.
2. Пространственная иерархия должна быть отделена от отображаемого содержимого.
3. Экземпляр объекта сцены должен быть отделён от тяжёлого ресурса.
4. UI должен работать только через высокоуровневый публичный API движка.
5. Низкоуровневый renderer не должен знать о UI.
6. `Scene` не должна зависеть от конкретного GPU backend.
7. Основные зависимости между модулями должны быть однонаправленными.
8. Циклические зависимости между архитектурными модулями не допускаются.
9. Обратная передача данных должна происходить через возвращаемые значения, события или специализированные структуры данных.
10. Публичный API должен оперировать объектами предметной области, а не низкоуровневыми GPU-командами.

---

# 3. Главная архитектурная схема

```text
              UI / Editor / Client Application
                            |
                            v
                      Public Engine API
                            |
                            v
                          Engine
              +-------------+-------------+
              |             |             |
              v             v             v
            Scene       Resources       Systems
              |             |             |
              |             |             +--- сборка, инициализация
              |             |             |    и завершение частей
              |             |             +--- загрузка моделей,
              |             |                  save/restore сцены
              v             |
      build_render_frame    |
              |             |
              v             v
          RenderFrame --> Renderer
                            |
                            v
                       RenderSystem
                            |
                            v
                    wgpu (backend) --> GPU
```

`Systems` собирает, инициализирует и завершает части движка (`systems::lifecycle`), загружает модели и хранит сохранённую сцену. Сборку, инициализацию и завершение выполняет `systems::lifecycle`; порядок шагов рабочего цикла задаёт главный цикл движка `EngineLoop` в `core` (§4.6).

---

# 4. Модуль `core`

## 4.1. Назначение

`core` содержит главный объект движка — `Engine`.

`Engine` является фасадом и оркестратором.

Он связывает:

- сцену;
- ресурсы;
- системы;
- renderer;
- lifecycle кадра.

## 4.2. Что делает `Engine`

`Engine` должен:

- инициализировать и завершать подсистемы (через `systems::lifecycle`);
- владеть или координировать основные модули;
- предоставлять публичную точку входа;
- владеть главным циклом движка (`EngineLoop`, §4.6) и порядком его шагов;
- выполнять `update`;
- инициировать `render`;
- управлять lifecycle движка;
- передавать данные между подсистемами.

## 4.3. Что `Engine` не должен делать

`Engine` не должен:

- самостоятельно хранить vertex/index data;
- самостоятельно создавать GPU draw calls;
- содержать UI;
- содержать бизнес-логику конкретного приложения;
- заменять собой SceneManager, ResourceManager или Renderer.

## 4.4. Предлагаемая структура

```rust
pub struct Engine {
    scene: SceneManager,
    resources: ResourceManager,
    renderer: Renderer,
    systems: SystemManager,
    event_loop: EngineLoop,
}
```

Поле называется `event_loop`, а не `loop`: `loop` — ключевое слово Rust и не может быть именем поля.

## 4.5. Минимальный публичный API

```rust
impl Engine {
    pub fn new() -> Self;

    // Публичные операции — трейт EngineApi (FREAK_Engine_Public_API.md);
    // initialize/shutdown выполняются через systems::lifecycle
    // и не являются событиями цикла (§4.6).

    // Шаги главного цикла движка (EngineLoop, §4.6):
    fn update(&mut self, dt: Duration) -> Result<(), EngineError>;
    fn render(&mut self) -> Result<(), EngineError>;
}
```

## 4.6. Главный цикл движка (`EngineLoop`)

`core` содержит главный цикл движка — `EngineLoop`

`EngineLoop` — один последовательный цикл. Он владеет порядком шагов и не является шиной событий (event bus): публикация и подписка, произвольные подписчики и асинхронная доставка не вводятся (см. §35).

Цикл работает только в состоянии `Ready` (§39) и не выполняет инициализацию и завершение: циклу нужны уже собранные и инициализированные части. Их собирает, инициализирует и завершает `systems::lifecycle` (`create`, `initialize`, `shutdown`, §18) по вызову `Engine::initialize`/`Engine::shutdown`; `EngineLoop` в этих переходах не участвует.

Один проход цикла:

```text
применить поступившие запросы клиента к сцене и ресурсам
        |
        v
update(dt) — обновление состояния
        |
        v
build_render_frame() — сборка кадра
        |
        v
render() — отрисовка через Renderer/RenderSystem
        |
        v
доставить SceneImage получателю (ImageListener)
        |
        v
ожидание следующего прохода (Tick)
```

Входные события цикла описаны типом `EngineEvent`. Основные события:

| Группа         | Событие                                                            | Источник                                  | Действие цикла                                     |
| -------------- | ------------------------------------------------------------------ | ----------------------------------------- | -------------------------------------------------- |
| Запрос клиента | `CreateNode`, `SetNodeTransform`, `ReparentObject`, `SetViewpoint` | `SceneApi`                                | применить изменение к сцене                        |
| Запрос клиента | `LoadModel(path, parent)`                                          | `EngineApi::load_model`                   | файловая операция `systems::loading`               |
| Запрос клиента | `SetImageSize(size)`                                               | `EngineApi::set_image_size`               | изменить размер изображения                        |
| Запрос клиента | `SaveScene`, `RestoreScene`                                        | `EngineApi::save_scene` / `restore_scene` | файловая операция `systems::persistence`           |
| Кадр           | `Tick(dt)`                                                         | сам цикл                                  | выполнить проход: `update`, сборка кадра, `render` |

Уведомление `ImageListener::image_ready(&SceneImage)` — результат прохода, а не вариант `EngineEvent`: цикл вызывает получателя напрямую (§21, §31).

```rust
pub struct EngineLoop {
    events: VecDeque<EngineEvent>,
    running: bool,
}

/// Входные события главного цикла: запросы клиента и шаги кадра.
pub enum EngineEvent {
    // Запросы клиента
    CreateNode { parent: SceneNodeId },
    SetNodeTransform { node: SceneNodeId, transform: Transform },
    ReparentObject { object: SceneObjectId, new_parent: SceneNodeId },
    SetViewpoint(ViewpointState),
    LoadModel { path: PathBuf, parent: SceneNodeId },
    SetImageSize(ImageSize),
    SaveScene,
    RestoreScene,

    // Кадр
    Tick(Duration),
}
```

Правила:

- запросы клиента применяются в том порядке, в котором они поступили; кадр строится по состоянию на начало прохода;
- ошибки шага не передаются событиями: они возвращаются через `Result` соответствующей операции (`EngineError`, `SceneError`, `ModelLoadError`, `ScenePersistenceError`, `LifecycleError`, §18); ошибка шага не отменяет уже применённые запросы и не останавливает цикл, если ошибка не является неустранимой (§39);
- цикл — единственное место, где задаётся порядок вызовов подсистем в рабочем режиме (§42.5); модули `scene`, `resources`, `render` и `systems` о цикле не знают.

Фоновой многопоточности, асинхронного исполнения и собственных систем клиента в v1 нет: `EngineLoop` — простое последовательное исполнение (см. §35).

---

# 5. Модуль `scene`

## 5.1. Назначение

Модуль `scene` отвечает за:

> что существует в виртуальном мире и где оно расположено.

Он не должен выполнять GPU-команды.

## 5.2. Основные сущности

```text
SceneManager
SceneNode
Transform
SceneObject
Viewpoint
SceneNodeId
SceneObjectId
```

---

# 6. `SceneManager`

## 6.1. Ответственность

`SceneManager` является владельцем логической сцены.

Он должен:

- хранить корневой `SceneNode`;
- создавать nodes;
- создавать объекты сцены из загруженных моделей;
- прикреплять объекты к nodes и перепривязывать их к другим узлам;
- хранить единственную точку обзора (вне иерархии узлов);
- предоставлять traversal scene graph;
- подготавливать данные для RenderFrame.

## 6.2. Возможная структура

```rust
pub struct SceneManager {
    root: SceneNodeId,
    nodes: Vec<SceneNode>,
    objects: Vec<SceneObject>,
    viewpoint: ViewpointState,
}
```

В v1 ничего не удаляется, поэтому достаточно `Vec`; конкретное хранилище — внутренняя деталь, идентификаторы непрозрачны.

## 6.3. Главное правило

`SceneManager` знает структуру сцены, но не знает, как GPU рисует её содержимое.

---

# 7. `SceneNode`

## 7.1. Назначение

`SceneNode` отвечает за:

> WHERE — где находится объект.

Он описывает пространственную иерархию.

## 7.2. Возможная структура

```rust
pub struct SceneNode {
    parent: Option<SceneNodeId>,
    children: Vec<SceneNodeId>,
    objects: Vec<SceneObjectId>,
    local_transform: Transform,
}
```

## 7.3. В `SceneNode` должны находиться

```text
position
rotation
scale
parent
children
attached objects
```

## 7.4. В `SceneNode` не должны находиться

```text
Mesh
Texture
GPU Buffer
RenderPipeline
Shader Module
```

---

# 8. `Transform`

Преобразование — положение, поворот и масштаб.

```rust
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

`Transform` есть только у `SceneNode` (локальное преобразование в иерархии). `SceneObject` собственного преобразования не хранит — положение, поворот и масштаб объекта определяются узлом, к которому он привязан. Мировая трансформация вычисляется при обходе сцены и не хранится постоянно:

```text
world_transform = parent_world_transform * local_transform
```

# 9. `SceneObjectId`

В первой версии объект сцены один — объект загруженной модели. Идентификаторы непрозрачны для клиента и strongly typed:

```rust
pub struct SceneObjectId(/* private */ u32);
```

Точка обзора объектом сцены не является и в иерархию не входит (см. §11). Источники света и другие виды объектов (enum-расширение `SceneObjectId`) — после v1.

---

# 10. `SceneObject`

## 10.1. Назначение

`SceneObject` — экземпляр отображаемого объекта в сцене (объект загруженной модели).

Его семантика:

> WHAT — что находится в SceneNode.

## 10.2. Возможная структура

```rust
pub struct SceneObject {
    mesh: MeshHandle,
}
```

Собственного преобразования у объекта нет: положение, поворот и масштаб определяются узлом, к которому он привязан (§7, §8).

## 10.3. Главное правило

`SceneObject` не должен содержать vertex/index data напрямую.

Он должен ссылаться на ресурсы.

```text
SceneObject ---> MeshHandle ---> Mesh
```

Один `Mesh` может использоваться несколькими `SceneObject`.

Удаление, скрытие и подмена внешнего вида объекта — после v1 (ФТ-15, ФТ-16, ФТ-23).

---

# 11. `Viewpoint`

Точка обзора одна на сцену и управляется независимо от иерархии узлов (ФТ-18).

```rust
pub struct ViewpointState {
    pub position: Vec3,
    pub direction: Vec3, // направление взгляда
    pub fov_y: f32,      // вертикальный угол обзора, радианы
}
```

Значения по умолчанию: позиция `(0, 0, 5)`, взгляд вдоль `-Z`, угол обзора 60°.

Ортографическая проекция и несколько точек обзора — после v1.

---

# 12. `Light`

```text
SceneNode
   |
   + Transform
   |
   + Light
```

```rust
pub struct Light {
    pub kind: LightType,
    pub color: Vec3,
    pub intensity: f32,
}
```

Положение источника света не должно храниться внутри `Light`.

**После v1.** В первой версии источников света нет (см. требования).

---

# 13. Модуль `resources`

## 13.1. Назначение

Resources отвечают за разделяемые данные моделей (в v1 — геометрия и общий простой цвет модели) и библиотеку встроенных мешей (§13.3).

## 13.2. Основные сущности

```text
ResourceManager
MeshLibrary
Mesh
SubMesh
Vertex
MeshHandle
```

`Material`, `Texture`, `Shader` — после v1.

## 13.3. Встроенные меши

Помимо мешей, загруженных из файлов, `resources` владеет библиотекой встроенных (готовых) мешей для быстрого создания стандартных геометрических фигур. Такие меши строятся средствами движка процедурно и не требуют файла модели.

- Встроенный меш устроен так же, как загруженный: `Mesh` + `SubMesh` + `Vertex` + `MeshHandle`; для сцены и рендера разницы нет.
- Форма может быть составной и сложной (несколько `SubMesh`), но остаётся одним `Mesh`.
- Конкретный перечень фигур, их разбиение на `SubMesh` и способ построения архитектурой не фиксируются — это решение разработчиков.
- Библиотеку хранит `ResourceManager` (§14) в виде `MeshLibrary`.
- Для клиента в v1 доступен только `load_model`; создание объекта из встроенного меша — после v1 (ФТ-23), как и ручное создание модели.

---

# 14. `ResourceManager`

```rust
pub struct ResourceManager {
    meshes: Vec<Mesh>,
    library: MeshLibrary,
}
```

Публичные handles:

```rust
MeshHandle
```

Handles должны быть strongly typed и непрозрачны для клиента. `MaterialHandle`/`TextureHandle`/`ShaderHandle` — после v1.

---

# 15. `Mesh`

`Mesh` является ресурсом геометрии и общего простого цвета модели.

```rust
pub struct Mesh {
    submeshes: Vec<SubMesh>,
    color: [f32; 4], // linear RGBA, общий цвет модели
}
```

```rust
pub struct SubMesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}
```

```rust
pub struct Vertex {
    position: Vec3,
}
```

Нормали и текстурные координаты — после v1 (вместе с освещением и текстурами).

---

# 16. Вне первой версии: `Material`, `Texture`, `Shader`

В первой версии внешний вид — один простой цвет модели, хранимый в `Mesh` (§15). Клиент не может переопределять внешний вид (ФТ-23).

После v1: `Material` (base color, затем metallic/roughness/normal map/emission/transparency), `Texture`, `Shader` как разделяемые ресурсы; handles `MaterialHandle`/`TextureHandle`/`ShaderHandle`.

---

# 17. Принцип Resource vs Instance

Это один из ключевых invariants системы.

```text
Mesh        != SceneObject
SceneNode   != SceneObject
```

Связь:

```text
SceneNode
    |
    + SceneObject
        |
        + MeshHandle --------> Mesh (геометрия + цвет модели)
```

`Material`, `Texture`, `Shader` добавляются после v1 по той же схеме (ресурс ≠ экземпляр).

Встроенные меши (§13.3) подчиняются тому же правилу: `Mesh` — ресурс, `SceneObject` — экземпляр; встроенный меш не отличается от загруженного.

---

# 18. Модуль `systems`

Systems отвечают за жизненный цикл и файловые операции: собирают, инициализируют и завершают части движка, загружают модели и сохраняют/восстанавливают сцену. Клиент не подключает собственные системы (см. требования).

```rust
pub struct EngineParts {
    pub scene: SceneManager,
    pub resources: ResourceManager,
    pub renderer: Renderer,
    pub store: SceneStore,
}

pub fn create() -> EngineParts;

pub fn initialize(
    parts: &mut EngineParts,
    image_size: ImageSize,
) -> Result<(), LifecycleError>;

pub fn shutdown(parts: &mut EngineParts) -> Result<(), LifecycleError>;

pub fn load_model(
    path: &Path,
    resources: &mut ResourceManager,
    scene: &mut SceneManager,
    parent: SceneNodeId,
) -> Result<SceneObjectId, ModelLoadError>;

pub struct SceneStore { /* private */ }

pub fn save_scene(
    store: &mut SceneStore,
    scene: &SceneManager,
    resources: &ResourceManager,
) -> Result<(), ScenePersistenceError>;

pub fn restore_scene(
    store: &mut SceneStore,
    scene: &mut SceneManager,
    resources: &mut ResourceManager,
) -> Result<(), ScenePersistenceError>;
```

`create` собирает части, `initialize` инициализирует графику и задаёт размер изображения, `shutdown` завершает работу; ошибки жизненного цикла — `LifecycleError`. В первой версии сохранение хранится в памяти движка; формат файла — после v1.

`System` trait не создаётся: заменяемые реализации систем не нужны, trait оправдан только там, где действительно нужна замена (см. §44), прежде всего для `RenderSystem`. Отсечение невидимого, анимационные и прочие системы — после v1.

## 18.1. Сериализация

За сериализацию состояния сцены отвечает `systems::persistence` — единственное место, которое знает о представлении сохраняемого состояния.

- Что сохраняется: полное состояние сцены — иерархия узлов, преобразования, привязки объектов, точка обзора и необходимые данные моделей (ФТ-26).
- Как: `persistence` строит отдельное сериализуемое представление (DTO) состояния сцены и ресурсов и работает с ним. Модули `scene` и `resources` о сериализации не знают и от неё не зависят.
- Формат: в v1 представление хранится в памяти (`SceneStore`); внешний формат (файл, кодеки) — после v1 и не влияет на остальные модули.

Загрузка модели (разбор glTF/OBJ) — другой случай: это десериализация внешних данных, и она живёт в `systems::loading`.

---

# 19. Frame Preparation

Между Scene и Renderer должна существовать отдельная граница данных.

Renderer не должен напрямую обходить внутренние структуры `SceneManager`.

Нужно формировать отдельную структуру `RenderFrame`.

---

# 20. `RenderFrame`

```rust
pub struct RenderFrame {
    pub camera: RenderCamera,
    pub items: Vec<RenderItem>,
}
```

```rust
pub struct RenderItem {
    pub world_transform: Mat4,
    pub mesh: MeshHandle,
}
```

```rust
pub struct RenderCamera {
    pub view: Mat4,
    pub projection: Mat4,
}
```

`lights` и `material` в кадре — после v1.

---

# 21. Основной pipeline кадра

```text
Scene
  |
  | traversal
  v
Scene Graph
  |
  | world-transform calculation
  v
item collection
  |
  v
RenderFrame
  |
  v
Renderer
  |
  v
RenderSystem
  |
  v
GPU
```

---

# 22. Renderer

Renderer отвечает за:

> HOW — как превратить RenderFrame в изображение.

Renderer не должен:

- менять Scene;
- знать о UI;
- принимать пользовательские команды;
- хранить бизнес-логику;
- знать, почему объект оказался в конкретной позиции.

```rust
pub struct Renderer {
    render_system: Box<dyn RenderSystem>,
}
```

---

# 23. `RenderSystem`

`RenderSystem` является интерфейсом между renderer и конкретным GPU backend.

В первой версии рендеринг offscreen: backend рисует в собственный буфер и возвращает `RenderOutput` (статистика + `SceneImage`). Окна и поверхности (`RenderSurface`) в v1 нет.

```rust
pub trait RenderSystem {
    fn initialize(&mut self) -> Result<(), RenderError>;

    fn resize(&mut self, size: ImageSize) -> Result<(), RenderError>;

    fn render(
        &mut self,
        frame: &RenderFrame,
        resources: &ResourceManager,
    ) -> Result<RenderOutput, RenderError>;

    fn shutdown(&mut self);
}
```

```rust
pub struct RenderOutput {
    pub stats: RenderStats,
    pub image: SceneImage,
}
```

---

# 24. GPU backend

Конкретные реализации:

```text
RenderSystem
      ^
      |
+-----+--------+
|              |
WgpuBackend   VulkanBackend (после v1)
```

На первом этапе следует реализовать только один backend — `wgpu`.

Архитектура должна позволять добавить другой backend позже, но не нужно проектировать чрезмерно сложный слой заранее.

---

# 25. Public API

UI не должен иметь доступ к GPU API.

UI должен видеть:

```text
Engine
Scene API
загрузка моделей (load_model)
high-level settings (размер изображения и т.п.)
```

UI не должен видеть:

```text
Device
Queue
CommandEncoder
CommandBuffer
BindGroup
Pipeline
DescriptorSet
VkDevice
```

---

# 26. Scene API

```rust
pub trait SceneApi {
    fn root_node(&self) -> SceneNodeId;

    fn create_node(
        &mut self,
        parent: SceneNodeId,
    ) -> Result<SceneNodeId, SceneError>;

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

    fn set_viewpoint(
        &mut self,
        state: ViewpointState,
    ) -> Result<(), SceneError>;
}
```

Внутренние операции `SceneManager` для движка (не клиентский API): `create_object(mesh, parent)` и `build_render_frame(size) -> RenderFrame`. Удаление узлов и объектов — после v1.

---

# 27. Resource API

У клиента нет отдельного Resource API: модель загружается через `EngineApi::load_model`. Внутренний интерфейс `ResourceManager`:

```rust
impl ResourceManager {
    pub(crate) fn add_mesh(&mut self, mesh: Mesh) -> MeshHandle;
    pub(crate) fn mesh(&self, handle: MeshHandle) -> Option<&Mesh>;
}
```

Библиотека встроенных мешей (§13.3) доступна внутри движка через тот же интерфейс; её содержимое — внутренняя деталь.

Загрузка текстур и создание материалов — после v1.

---

# 28. Типичный клиентский код

```rust
let mut engine = Engine::new();
engine.initialize(ImageSize::new(1280, 720))?;
engine.set_image_listener(Box::new(AppListener { /* ... */ }));

let root = engine.scene().root_node();
let node = engine.scene().create_node(root)?;

engine.load_model(Path::new("cube.gltf"), node)?;
engine.scene().set_node_transform(
    node,
    Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
)?;

// Отрисовкой управляет сам движок; клиент получает изображения
// через ImageListener и не вызывает update/render.
engine.shutdown()?;
```

---

# 29. Dependency rules

Разрешённые направления зависимостей:

```text
UI -> Engine API

core -> scene, resources, systems, render

scene -> math, resources, render::frame (только DTO кадра)

render -> math, resources

backend -> render, resources, math

systems -> scene, resources, render, backend
```

Запрещённые зависимости:

```text
Scene -> Renderer
Scene -> RenderSystem
Scene -> wgpu

Resources -> UI

Renderer -> UI
Renderer -> SceneManager

RenderSystem -> SceneManager

Mesh -> SceneManager

GPU Backend -> Engine
```

---

# 30. Обратная связь между модулями

Не создавать двусторонние ownership-связи.

Плохо:

```rust
struct Scene {
    renderer: Renderer,
}

struct Renderer {
    scene: Scene,
}
```

Правильно использовать:

- аргументы методов;
- `Result`;
- события;
- immutable/mutable references;
- специализированные DTO/data structures.

Например:

```rust
let frame = scene.build_render_frame(...);

let output = renderer.render(
    &frame,
    &resources,
)?; // RenderOutput { stats, image }
```

---

# 31. Lifecycle одного кадра

`update`/`render` — внутренние шаги главного цикла движка (`EngineLoop`, §4.6); клиент их не вызывает.

```text
Idle
 |
 | Engine::update(dt) (внутренний шаг)
 v
Обновление подсистем
 |
 v
Scene updated
 |
 | Engine::render() (внутренний шаг)
 v
Scene traversal
 |
 v
World transform calculation
 |
 v
Item collection
 |
 v
Build RenderFrame
 |
 v
Renderer::render() -> RenderSystem draw commands
 |
 v
SceneImage -> ImageListener
 |
 v
Idle
```

---

# 32. Lifecycle объекта сцены

```text
Created
   |
   | attach (при загрузке модели)
   v
Attached
   |
   | set_node_transform / reparent_object
   v
Attached
```

Удаление, скрытие и состояние Detached — после v1 (ФТ-15, ФТ-16). API должен корректно обрабатывать некорректные переходы (неизвестный ID, перепривязка узла в собственного потомка).

---

# 33. Предлагаемая структура crate

На первом этапе желательно использовать один crate:

```text
freak-engine/   (крейт BFGE)
├── Cargo.toml
└── src/
    ├── lib.rs              # публичная поверхность (ре-экспорты)
    ├── math.rs             # ре-экспорт glam: Vec3, Quat, Mat4
    │
    ├── core/
    │   ├── mod.rs
    │   ├── api.rs          # EngineApi, ImageListener
    │   ├── engine.rs       # Engine
    │   ├── engine_loop.rs  # EngineLoop, EngineEvent
    │   └── error.rs        # EngineError
    │
    ├── scene/
    │   ├── mod.rs
    │   ├── api.rs          # SceneApi
    │   ├── ids.rs          # SceneNodeId, SceneObjectId
    │   ├── transform.rs    # Transform
    │   ├── node.rs         # SceneNode
    │   ├── object.rs       # SceneObject
    │   ├── viewpoint.rs    # ViewpointState
    │   ├── manager.rs      # SceneManager (+ build_render_frame)
    │   └── error.rs        # SceneError
    │
    ├── resources/
    │   ├── mod.rs
    │   ├── handle.rs       # MeshHandle
    │   ├── mesh.rs         # Vertex, SubMesh, Mesh
    │   ├── library.rs      # MeshLibrary (встроенные меши, §13.3)
    │   └── manager.rs      # ResourceManager
    │
    ├── systems/
    │   ├── mod.rs
    │   ├── lifecycle.rs    # EngineParts, create/initialize/shutdown, LifecycleError
    │   ├── loading.rs      # load_model, ModelLoadError
    │   └── persistence.rs  # SceneStore, save/restore, сериализация сцены (§18.1), ScenePersistenceError
    │
    ├── render/
    │   ├── mod.rs
    │   ├── frame.rs        # RenderFrame, RenderCamera, RenderItem
    │   ├── renderer.rs     # Renderer
    │   ├── system.rs       # RenderSystem, RenderOutput, RenderError
    │   ├── stats.rs        # RenderStats
    │   └── image.rs        # ImageSize, SceneImage
    │
    └── backend/
        ├── mod.rs
        └── wgpu/
            ├── mod.rs
            ├── backend.rs  # WgpuRenderSystem
            ├── pipeline.rs
            ├── buffer.rs
            └── texture.rs
```

---

# 34. UI как отдельный crate

Если имеется редактор, его следует отделить:

```text
workspace/
├── freak-engine/
└── freak-editor/
```

`freak-editor` зависит от `freak-engine`.

`freak-engine` не должен зависеть от редактора.

---

# 35. Что не нужно реализовывать в первой версии

Не следует преждевременно внедрять:

- полноценный ECS;
- render graph;
- frame graph;
- plugin framework;
- multithreaded renderer;
- async resource streaming;
- hot reload;
- LOD;
- instancing;
- batching;
- dependency injection framework;
- event bus;
- универсальную backend abstraction на множество API;
- сложную иерархию trait objects.

Подготовить архитектуру так, чтобы такие возможности можно было добавить позже.

---

# 36. MVP

Минимальная рабочая версия должна содержать:

```text
Engine, EngineLoop, EngineEvent, EngineApi, ImageListener

SceneManager, SceneApi
SceneNode, SceneObject
Transform, ViewpointState

Mesh, MeshHandle
ResourceManager (разделяемые меши, включая библиотеку встроенных мешей — §13.3)

RenderFrame, RenderItem, RenderCamera

Renderer, RenderSystem, WgpuRenderSystem (скелет)

systems::lifecycle (сборка/инициализация/завершение частей), load_model (glTF/OBJ — интерфейс), save/restore сцены (в памяти, сериализация — §18.1)
```

MVP должен позволять:

```text
создать и инициализировать Engine
создать SceneNode
создать SceneObject из загруженной модели (load_model)
изменить Transform узла (объект следует за узлом)
перепривязать объект к другому узлу
задать точку обзора
сформировать RenderFrame
отрисовать кадр и получить SceneImage
сохранить и восстановить сцену (в памяти)
```

---

# 37. UML class diagram

Файл: `docs/arch.png` (перегенерировать из блока ниже после правок).

```plantuml
@startuml

package Core {
    class Engine
    class EngineLoop
    class EngineEvent
}

package Scene {
    class SceneManager
    class SceneNode
    class SceneObject
    class Viewpoint

    class Transform
    class SceneNodeId
    class SceneObjectId
}

package Resources {
    class ResourceManager
    class MeshLibrary
    class Mesh
    class SubMesh
    class Vertex
    class MeshHandle
}

package Rendering {
    class RenderFrame
    class RenderItem
    class RenderCamera

    interface RenderSystem
    class Renderer
    class ImageSize
    class SceneImage
}

Engine --> SceneManager
Engine --> ResourceManager
Engine --> Renderer
Engine ..> RenderSystem
Engine --> EngineLoop
EngineLoop ..> EngineEvent

SceneManager *-- SceneNode
SceneManager *-- Viewpoint
SceneNode --> SceneObject : objects
SceneObject --> MeshHandle
SceneNode *-- Transform

ResourceManager *-- Mesh
ResourceManager *-- MeshLibrary
Mesh *-- SubMesh
SubMesh *-- Vertex

SceneManager ..> RenderFrame : builds
RenderFrame *-- RenderItem
RenderFrame *-- RenderCamera

Renderer --> RenderSystem
Renderer ..> RenderFrame
Renderer --> ResourceManager

@enduml
```

---

# 38. UML sequence diagram одного кадра

Файл: `docs/architecture/frame-sequence.puml`

```plantuml
@startuml

participant UI
participant Engine
participant EngineLoop
participant SceneManager
participant ResourceManager
participant Renderer
participant RenderSystem
participant GPU

UI -> Engine : initialize / shutdown
Engine -> Engine : systems::lifecycle (create / initialize / shutdown)

UI -> Engine : load_model / set_* / save / restore
Engine -> EngineLoop : событие запроса (EngineEvent)

== главный цикл движка (EngineLoop) ==

EngineLoop -> Engine : Tick(dt)
Engine -> SceneManager : build_render_frame(size)
SceneManager -> SceneManager : обход графа и мировые трансформации
SceneManager --> Engine : RenderFrame

Engine -> Renderer : render(frame, resources)
Renderer -> ResourceManager : resolve handles
Renderer -> RenderSystem : render(frame, resources)
RenderSystem -> GPU : draw commands
GPU --> RenderSystem : image
RenderSystem --> Renderer : RenderOutput { stats, image }
Renderer --> Engine : RenderOutput

Engine -> UI : image_ready(&SceneImage)

@enduml
```

---

# 39. UML state diagram Engine

Файл: `docs/architecture/engine-state.puml`

```plantuml
@startuml

[*] --> Created

Created --> Initializing : Engine::new() / initialize(size)

Initializing --> Ready
Initializing --> Failed

Ready --> Updating : update(dt)
Updating --> Ready

Ready --> PreparingFrame : render() (внутренний шаг цикла)

PreparingFrame --> Rendering
Rendering --> Presenting
Presenting --> Ready

Ready --> Resizing : set_image_size()
Resizing --> Ready

Rendering --> Failed : unrecoverable error

Ready --> ShuttingDown
Failed --> ShuttingDown

ShuttingDown --> Terminated

Terminated --> [*]

@enduml
```

---

# 40. UML state diagram объекта сцены

Файл: `docs/architecture/object-state.puml`

```plantuml
@startuml

[*] --> Attached : load_model(path, parent)

Attached --> Attached : set_node_transform(...)
Attached --> Attached : reparent_object(node)

@enduml
```

Удаление и скрытие — после v1 (ФТ-15, ФТ-16).

---

# 41. UML component diagram

Файл: `docs/architecture/components.puml`

```plantuml
@startuml

component UI

component "Public API" as API
component "Engine / Core" as Core

component "Scene" as Scene
component "Systems" as Systems
component "Resources" as Resources

component "Renderer" as Renderer
component "Graphics Backend" as Backend

UI --> API

API --> Core

Core --> Scene
Core --> Systems
Core --> Resources
Core --> Renderer

Systems --> Scene
Systems --> Resources
Systems --> Renderer : собирает и инициализирует
Systems --> Backend : создаёт

Scene --> Resources
Scene ..> Renderer : RenderFrame

Renderer --> Resources
Renderer --> Backend

@enduml
```

---

# 42. Формальное описание границ

## 42.1. Scene

Scene отвечает за:

```text
что существует
где это находится
как объекты связаны иерархически
```

Scene не отвечает за:

```text
GPU commands
pipelines
swapchain
command buffers
UI
```

## 42.2. Resources

Resources отвечают за:

```text
какие повторно используемые данные существуют
```

Примеры:

```text
Mesh (геометрия + цвет модели)
MeshHandle
встроенные меши стандартных фигур (MeshLibrary, §13.3)
```

Текстуры, материалы, шейдеры — после v1.

## 42.3. Systems

Systems отвечают за:

```text
сборку, инициализацию и завершение частей движка;
загрузку моделей и save/restore сцены (включая сериализацию состояния сцены — §18.1)
```

## 42.4. Renderer

Renderer отвечает за:

```text
как подготовленный RenderFrame превращается в изображение (offscreen: RenderOutput со SceneImage)
```

## 42.5. Engine

Engine отвечает за:

```text
когда и в каком порядке вызываются подсистемы
```

Главный цикл движка (`EngineLoop`, §4.6) — конкретная реализация этого порядка в рабочем режиме внутри `core`; сборку, инициализацию и завершение частей выполняет `systems::lifecycle` (§18).

## 42.6. UI

UI отвечает за:

```text
что пользователь хочет сделать
```

---

# 43. Главные invariants

1. `SceneNode` отвечает за **WHERE**.
2. `SceneObject` отвечает за **WHAT**.
3. `Mesh` является resource, `SceneObject` — instance.
4. Scene не выполняет GPU-команды.
5. Renderer не изменяет Scene.
6. RenderSystem не знает о SceneManager.
7. UI не знает о GPU backend.
8. Engine является orchestrator/facade.
9. Между Scene и Renderer передаётся `RenderFrame`.
10. Один `Mesh` может использоваться несколькими `SceneObject`.
11. `Transform` есть только у `SceneNode`; положение объекта определяется его узлом, мировая трансформация вычисляется при сборке кадра.
12. Точка обзора одна и не входит в иерархию узлов.
13. Resource handles должны быть strongly typed.
14. Клиент не удаляет и не скрывает объекты и не переопределяет внешний вид (v1).
15. Dependency graph не должен иметь циклов.
16. Встроенный меш — такой же `Mesh`-ресурс, как загруженный; для сцены и рендера разницы нет.
17. Состояние сцены сериализует только `systems::persistence`; `scene` и `resources` не знают о формате сохранения.
18. Главный цикл движка один; он задаёт порядок применения запросов, обновления и отрисовки и не выполняет инициализацию и завершение (это `systems::lifecycle`).

---

# 44. Инструкция Codex-агенту

Сначала создать архитектурный skeleton проекта:

- module declarations;
- основные типы;
- strongly typed IDs;
- публичные API;
- trait boundaries;
- `RenderFrame`;
- ошибки;
- UML-документацию.

Не реализовывать полноценный GPU renderer до завершения архитектурного skeleton.

Все публичные сущности должны иметь Rustdoc.

Для ещё не реализованной логики допустимо использовать `todo!()`, если это позволяет сохранить целевые сигнатуры и архитектурные связи.

Не добавлять новые архитектурные сущности без необходимости.

Не внедрять в первой версии:

- ECS framework;
- event bus;
- plugin framework;
- dependency injection framework;
- render graph;
- сложную trait hierarchy.

Предпочитать:

- конкретные Rust-типы;
- enums;
- strongly typed handles;
- ownership через manager-объекты;

сложным иерархиям `dyn Trait`.

Trait использовать там, где действительно нужна заменяемая реализация, прежде всего для `RenderSystem`.

Главная цель первой итерации:

> получить чистые границы модулей, компилируемый публичный API и понятную модель взаимодействия, а не полноценный production renderer.

---

# 45. Краткая архитектурная формула

```text
Scene знает, ЧТО существует и ГДЕ оно находится.

SceneNode знает WHERE.

SceneObject знает WHAT.

Mesh и MeshHandle знают, КАКИЕ разделяемые данные существуют.

Systems знают, КАК собрать, инициализировать и завершить части движка, загрузить модель и сохранить/восстановить сцену (включая её сериализацию).

EngineLoop знает, КОГДА и В КАКОМ ПОРЯДКЕ применяются запросы, обновляется состояние и строится кадр.

build_render_frame знает, ЧТО должно попасть в конкретный кадр.

Renderer знает, КАК превратить кадр в GPU-команды.

RenderSystem знает, КАК выполнить эти команды через конкретный backend.

Engine знает, КОГДА и В КАКОМ ПОРЯДКЕ вызвать подсистемы.

UI знает, ЧТО пользователь хочет сделать.
```

---
