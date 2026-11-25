// Interface translation. The UI is written in English; this swaps the text on screen for Spanish or
// Portuguese as it is drawn (text nodes, placeholders, titles). Game titles, paths and logs are untouched
// because they never match an entry. To add a line: [English, Español, Português].

const T = [
  // ── navigation & common ──
  ["Home", "Inicio", "Início"],
  ["Library", "Biblioteca", "Biblioteca"],
  ["Settings", "Ajustes", "Configurações"],
  ["Developer", "Desarrollador", "Desenvolvedor"],
  ["Play", "Jugar", "Jogar"],
  ["Stop", "Detener", "Parar"],
  ["Details", "Detalles", "Detalhes"],
  ["Favorite", "Favorito", "Favorito"],
  ["Favorites", "Favoritos", "Favoritos"],
  ["Game settings", "Ajustes del juego", "Configurações do jogo"],
  ["TV mode", "Modo TV", "Modo TV"],
  ["Open", "Abrir", "Abrir"],
  ["Search", "Buscar", "Buscar"],
  ["Change", "Cambiar", "Alterar"],
  ["Adjust", "Ajustar", "Ajustar"],
  ["Back", "Atrás", "Voltar"],
  ["Run tests", "Ejecutar pruebas", "Rodar testes"],
  ["Clear log", "Limpiar registro", "Limpar log"],
  ["Switch tab", "Cambiar pestaña", "Trocar aba"],
  ["Fullscreen", "Pantalla completa", "Tela cheia"],
  ["Cancel", "Cancelar", "Cancelar"],
  ["Close", "Cerrar", "Fechar"],
  ["OK", "OK", "OK"],
  ["Apply", "Aplicar", "Aplicar"],
  ["Applied", "Aplicado", "Aplicado"],
  ["Apply again", "Aplicar de nuevo", "Aplicar de novo"],
  ["Apply these settings", "Aplicar estos ajustes", "Aplicar estas configurações"],
  ["Start", "Empezar", "Começar"],
  ["Remove", "Quitar", "Remover"],
  ["Replace", "Reemplazar", "Substituir"],
  ["Reset", "Restablecer", "Redefinir"],
  ["Restore", "Restaurar", "Restaurar"],
  ["Refresh", "Actualizar", "Atualizar"],
  ["Copy", "Copiar", "Copiar"],
  ["Clear", "Limpiar", "Limpar"],
  ["Browse", "Examinar", "Procurar"],
  ["Install", "Instalar", "Instalar"],
  ["Installed", "Instalado", "Instalado"],
  ["Download", "Descargar", "Baixar"],
  ["Download all", "Descargar todo", "Baixar tudo"],
  ["Update", "Actualizar", "Atualizar"],
  ["Later", "Más tarde", "Depois"],
  ["No thanks", "No, gracias", "Não, obrigado"],
  ["Try again", "Intentar de nuevo", "Tentar de novo"],
  ["Loading…", "Cargando…", "Carregando…"],
  ["Checking…", "Comprobando…", "Verificando…"],
  ["Counting…", "Contando…", "Contando…"],
  ["Ready", "Listo", "Pronto"],
  ["Paused", "En pausa", "Pausado"],
  ["None", "Ninguno", "Nenhum"],
  ["Off", "Apagado", "Desligado"],
  ["Auto", "Auto", "Auto"],
  ["Automatic", "Automático", "Automático"],
  ["Automatic (Windows)", "Automático (Windows)", "Automático (Windows)"],
  ["Automatic (recommended)", "Automático (recomendado)", "Automático (recomendado)"],
  ["Default", "Predeterminado", "Padrão"],
  ["Unknown", "Desconocido", "Desconhecido"],
  ["Not set", "Sin definir", "Não definido"],
  ["Not installed", "No instalado", "Não instalado"],
  ["Folder", "Carpeta", "Pasta"],
  ["Program", "Programa", "Programa"],
  ["Size", "Tamaño", "Tamanho"],
  ["Location", "Ubicación", "Local"],
  ["Version", "Versión", "Versão"],
  ["Status", "Estado", "Status"],
  ["Title", "Título", "Título"],
  ["Platform", "Plataforma", "Plataforma"],
  ["Game", "Juego", "Jogo"],
  ["Games", "Juegos", "Jogos"],
  ["Your games", "Tus juegos", "Seus jogos"],
  ["Engine", "Motor", "Motor"],
  ["Engines", "Motores", "Motores"],
  ["Graphics", "Gráficos", "Gráficos"],
  ["Processor", "Procesador", "Processador"],
  ["Session", "Sesión", "Sessão"],
  ["Launcher", "Lanzador", "Lançador"],
  ["Preferences", "Preferencias", "Preferências"],
  ["Welcome", "Bienvenido", "Bem-vindo"],
  ["Choose file", "Elegir archivo", "Escolher arquivo"],
  ["Choose folder", "Elegir carpeta", "Escolher pasta"],
  ["Open folder", "Abrir carpeta", "Abrir pasta"],
  ["Earlier", "Antes", "Antes"],
  ["Under a minute", "Menos de un minuto", "Menos de um minuto"],
  ["Never played", "Nunca jugado", "Nunca jogado"],
  ["Last played", "Última vez", "Jogado por último"],
  ["Recent", "Recientes", "Recentes"],
  ["Recently played", "Jugados recientemente", "Jogados recentemente"],
  ["All", "Todos", "Todos"],
  ["All games", "Todos los juegos", "Todos os jogos"],
  ["Grid", "Cuadrícula", "Grade"],
  ["List", "Lista", "Lista"],
  ["Stats", "Estadísticas", "Estatísticas"],
  ["Search title or ID", "Buscar título o ID", "Buscar título ou ID"],
  ["Any engine", "Cualquier motor", "Qualquer motor"],
  ["Works", "Funciona", "Funciona"],
  ["Untested", "Sin probar", "Não testado"],
  ["Failed", "Falló", "Falhou"],
  ["WORKS", "FUNCIONA", "FUNCIONA"],
  ["FAILED", "FALLÓ", "FALHOU"],
  ["Worked", "Funcionó", "Funcionou"],
  ["Boots", "Inicia", "Inicia"],
  ["Playable", "Jugable", "Jogável"],
  ["Broken", "Roto", "Quebrado"],
  ["How does it run?", "¿Cómo corre?", "Como roda?"],
  ["Notes — what works, which settings helped…", "Notas — qué funciona, qué ajustes ayudaron…", "Notas — o que funciona, quais configurações ajudaram…"],
  ["Added to favorites.", "Añadido a favoritos.", "Adicionado aos favoritos."],
  ["Removed from favorites.", "Quitado de favoritos.", "Removido dos favoritos."],
  ["Nothing matches.", "No hay resultados.", "Nada encontrado."],
  ["Your shelf is empty.", "Tu estante está vacío.", "Sua estante está vazia."],
  ["Put game folders in the Games folder, or add your own folder.", "Pon las carpetas de los juegos en la carpeta Games, o añade tu propia carpeta.", "Coloque as pastas dos jogos na pasta Games ou adicione sua própria pasta."],
  ["No games yet. Add a folder in Settings → Library.", "Aún no hay juegos. Añade una carpeta en Ajustes → Biblioteca.", "Ainda não há jogos. Adicione uma pasta em Configurações → Biblioteca."],
  ["Copy a game folder (the one that contains eboot.bin) into your games folder, or add the folder where your games already are. Strata finds them automatically.", "Copia la carpeta de un juego (la que tiene eboot.bin) en tu carpeta de juegos, o añade la carpeta donde ya están tus juegos. Strata los encuentra solo.", "Copie a pasta de um jogo (a que tem o eboot.bin) para a sua pasta de jogos, ou adicione a pasta onde seus jogos já estão. O Strata encontra sozinho."],
  ["Open games folder", "Abrir carpeta de juegos", "Abrir pasta de jogos"],
  ["Scan again", "Buscar de nuevo", "Procurar de novo"],
  ["No game running.", "No hay ningún juego abierto.", "Nenhum jogo aberto."],
  ["Try next engine", "Probar el siguiente motor", "Tentar o próximo motor"],
  ["Close this engine and try the next one", "Cerrar este motor y probar el siguiente", "Fechar este motor e tentar o próximo"],
  ["Time played", "Tiempo jugado", "Tempo jogado"],
  ["Most played", "Más jugados", "Mais jogados"],
  ["Most used engine", "Motor más usado", "Motor mais usado"],
  ["Launches", "Inicios", "Inicializações"],
  ["Made with", "Hecho con", "Feito com"],
  ["Nothing played yet.", "Aún no has jugado nada.", "Nada jogado ainda."],
  ["Play a game to see it", "Juega algo para verlo aquí", "Jogue algo para ver aqui"],
  ["Hardware profile", "Perfil del hardware", "Perfil do hardware"],
  ["Hardware unknown", "Hardware desconocido", "Hardware desconhecido"],
  ["Checking PC…", "Revisando el PC…", "Verificando o PC…"],
  ["Checking your PC…", "Revisando tu PC…", "Verificando seu PC…"],
  ["Controller", "Control", "Controle"],
  ["Open the download page", "Abrir la página de descarga", "Abrir a página de download"],

  // ── settings sections ──
  ["Setup", "Configuración", "Configuração"],
  ["This PC", "Este PC", "Este PC"],
  ["Display", "Pantalla", "Tela"],
  ["Rendering", "Renderizado", "Renderização"],
  ["Performance", "Rendimiento", "Desempenho"],
  ["System", "Sistema", "Sistema"],
  ["Community", "Comunidad", "Comunidade"],
  ["Saves & backups", "Partidas y copias", "Saves e backups"],
  ["Appearance", "Apariencia", "Aparência"],
  ["About", "Acerca de", "Sobre"],
  ["Compatibility", "Compatibilidad", "Compatibilidade"],
  ["Update, DLC & mods", "Actualización, DLC y mods", "Atualização, DLC e mods"],
  ["← Global settings", "← Ajustes generales", "← Configurações gerais"],
  ["Back to global settings", "Volver a los ajustes generales", "Voltar às configurações gerais"],
  ["Back to the global settings.", "De vuelta a los ajustes generales.", "De volta às configurações gerais."],
  ["Use global settings again", "Usar los ajustes generales otra vez", "Usar as configurações gerais de novo"],
  ["Custom settings", "Ajustes propios", "Configurações próprias"],
  ["Uses its own settings", "Usa sus propios ajustes", "Usa configurações próprias"],
  ["Different from the global setting", "Distinto del ajuste general", "Diferente da configuração geral"],
  ["This game follows the global settings. Turn on custom settings to change them for this game only.", "Este juego usa los ajustes generales. Activa los ajustes propios para cambiarlos solo en este juego.", "Este jogo segue as configurações gerais. Ligue as configurações próprias para mudar só neste jogo."],
  ["Reset this page to defaults", "Restablecer esta página", "Redefinir esta página"],
  ["Reset to defaults.", "Valores predeterminados restaurados.", "Padrões restaurados."],
  ["This game", "Este juego", "Este jogo"],

  // ── This PC / profile ──
  ["Strata reads your hardware and picks settings that fit. You can re-run it any time; everything stays editable.", "Strata lee tu hardware y elige ajustes que encajan. Puedes repetirlo cuando quieras; todo sigue siendo editable.", "O Strata lê seu hardware e escolhe configurações que combinam. Dá pra repetir quando quiser; tudo continua editável."],
  ["Reading hardware…", "Leyendo el hardware…", "Lendo o hardware…"],
  ["Could not read the hardware. Check Developer → Logs.", "No se pudo leer el hardware. Revisa Desarrollador → Registros.", "Não foi possível ler o hardware. Veja Desenvolvedor → Logs."],
  ["Detect again", "Detectar de nuevo", "Detectar de novo"],
  ["Recommended for this PC", "Recomendado para este PC", "Recomendado para este PC"],
  ["Update your graphics driver.", "Actualiza el driver de tu tarjeta gráfica.", "Atualize o driver da placa de vídeo."],
  ["All graphics cards", "Todas las tarjetas gráficas", "Todas as placas de vídeo"],

  // ── Display ──
  ["Window size and how frames reach the screen.", "Tamaño de la ventana y cómo llegan los cuadros a la pantalla.", "Tamanho da janela e como os quadros chegam à tela."],
  ["Resolution", "Resolución", "Resolução"],
  ["Window and output size for KytyPS5 and SharpEmu. Higher sizes look sharper but cost performance. 1280×720 is the safest on laptops.", "Tamaño de ventana y salida para KytyPS5 y SharpEmu. Más grande se ve mejor pero cuesta rendimiento. 1280×720 es lo más seguro en laptops.", "Tamanho da janela e da saída no KytyPS5 e no SharpEmu. Maior fica mais nítido, mas custa desempenho. 1280×720 é o mais seguro em notebooks."],
  ["Presentation", "Presentación", "Apresentação"],
  ["Mailbox — smooth, no tearing", "Mailbox — fluido, sin cortes", "Mailbox — suave, sem cortes"],
  ["Immediate — fastest, may tear", "Immediate — lo más rápido, puede cortar", "Immediate — o mais rápido, pode cortar"],
  ["Fifo — locked to refresh rate", "Fifo — fijo a la frecuencia del monitor", "Fifo — preso à taxa do monitor"],
  ["Immediate gives the most frames; Fifo adds input delay.", "Immediate da más cuadros; Fifo añade retraso en los controles.", "Immediate dá mais quadros; Fifo adiciona atraso nos controles."],
  ["Graphics card", "Tarjeta gráfica", "Placa de vídeo"],
  ["On laptops pick the dedicated GPU, never the integrated one.", "En laptops elige la GPU dedicada, nunca la integrada.", "Em notebooks escolha a GPU dedicada, nunca a integrada."],
  ["Borderless fullscreen on the current monitor.", "Pantalla completa sin bordes en el monitor actual.", "Tela cheia sem bordas no monitor atual."],
  ["Stretch to window", "Estirar a la ventana", "Esticar na janela"],
  ["Fill the window instead of keeping the aspect ratio.", "Llena la ventana en vez de mantener la proporción.", "Preenche a janela em vez de manter a proporção."],
  ["HDR output", "Salida HDR", "Saída HDR"],
  ["Only for HDR monitors with HDR enabled in Windows.", "Solo para monitores HDR con HDR activado en Windows.", "Só para monitores HDR com HDR ligado no Windows."],
  ["Hide mouse cursor", "Ocultar el cursor", "Esconder o cursor"],
  ["Hides the cursor after 2 seconds without movement.", "Oculta el cursor tras 2 segundos sin moverse.", "Esconde o cursor depois de 2 segundos parado."],
  ["Virtual refresh rate", "Frecuencia virtual", "Taxa de atualização virtual"],
  ["Auto (match monitor)", "Auto (igual al monitor)", "Auto (igual ao monitor)"],
  ["Auto matches the monitor Strata is on (144 Hz, 280 Hz…). If a game runs too fast, set this game to 60 Hz.", "Auto usa la frecuencia del monitor donde está Strata (144 Hz, 280 Hz…). Si un juego va demasiado rápido, ponlo en 60 Hz.", "Auto usa a taxa do monitor onde o Strata está (144 Hz, 280 Hz…). Se um jogo ficar rápido demais, coloque 60 Hz nele."],
  ["Performance overlay", "Contador de rendimiento", "Contador de desempenho"],
  ["KytyPS5: FPS, frame time and memory (F2 toggles it). In shadPS4 and AnyPS5 press F10 while playing.", "KytyPS5: FPS, tiempo por cuadro y memoria (F2 lo muestra). En shadPS4 y AnyPS5 pulsa F10 mientras juegas.", "KytyPS5: FPS, tempo por quadro e memória (F2 mostra). No shadPS4 e no AnyPS5 aperte F10 durante o jogo."],

  // ── Rendering ──
  ["Speed versus accuracy. Leave fixes off unless a game shows black screens or missing textures.", "Velocidad contra precisión. Deja las correcciones apagadas salvo que un juego muestre pantallas negras o texturas que faltan.", "Velocidade contra precisão. Deixe as correções desligadas, a não ser que o jogo mostre tela preta ou texturas faltando."],
  ["Shader cache", "Caché de shaders", "Cache de shaders"],
  ["Saves compiled shaders, so the second run stutters much less. Keep on.", "Guarda los shaders compilados, así la segunda vez hay muchos menos tirones. Déjalo encendido.", "Guarda os shaders compilados, assim a segunda vez engasga bem menos. Deixe ligado."],
  ["Relaxed shader math", "Cálculo de shaders relajado", "Cálculo de shaders relaxado"],
  ["Faster on weak GPUs; colors or shadows can be slightly wrong.", "Más rápido en GPUs débiles; colores o sombras pueden verse algo distintos.", "Mais rápido em GPUs fracas; cores ou sombras podem ficar um pouco erradas."],
  ["Submit GPU work every N draws", "Enviar trabajo a la GPU cada N dibujos", "Enviar trabalho à GPU a cada N desenhos"],
  ["Can lower frame time in some games. 0 = off. Watch the overlay; set back to 0 if it gets worse.", "Puede bajar el tiempo por cuadro en algunos juegos. 0 = apagado. Mira el contador; vuelve a 0 si empeora.", "Pode baixar o tempo por quadro em alguns jogos. 0 = desligado. Veja o contador; volte a 0 se piorar."],
  ["Fixes black screens in some games. Costs speed.", "Corrige pantallas negras en algunos juegos. Cuesta velocidad.", "Corrige tela preta em alguns jogos. Custa velocidade."],
  ["Next step if linear readback did not help.", "Siguiente paso si el readback lineal no ayudó.", "Próximo passo se o readback linear não ajudou."],
  ["Last readback step; the heaviest.", "Último paso de readback; el más pesado.", "Último passo de readback; o mais pesado."],
  ["Accurate texture sync", "Sincronización precisa de texturas", "Sincronização precisa de texturas"],
  ["Fixes missing textures and wrong transparency. Costs speed.", "Corrige texturas que faltan y transparencias malas. Cuesta velocidad.", "Corrige texturas faltando e transparência errada. Custa velocidade."],
  ["Occlusion queries", "Consultas de oclusión", "Consultas de oclusão"],
  ["Keeps hidden lights and objects hidden. Costs speed.", "Mantiene ocultas las luces y objetos ocultos. Cuesta velocidad.", "Mantém escondidos luzes e objetos escondidos. Custa velocidade."],
  ["Tessellation", "Teselación", "Tesselação"],
  ["Experimental. Can crash games that use it.", "Experimental. Puede cerrar los juegos que la usan.", "Experimental. Pode fechar jogos que usam isso."],
  ["Native ray tracing", "Ray tracing nativo", "Ray tracing nativo"],
  ["Experimental. Needs an RT-capable GPU.", "Experimental. Necesita una GPU con RT.", "Experimental. Precisa de GPU com RT."],
  ["Enhanced console mode", "Modo consola mejorada", "Modo console aprimorado"],
  ["Experimental mode for games that support it. Required for upscaling.", "Modo experimental para juegos compatibles. Necesario para el escalado.", "Modo experimental para jogos compatíveis. Necessário para o upscaling."],
  ["Upscaler", "Escalado", "Upscaler"],
  ["FSR (any GPU)", "FSR (cualquier GPU)", "FSR (qualquer GPU)"],
  ["Renders smaller and scales up. DLSS and FSR need their files in a folder.", "Dibuja más pequeño y lo agranda. DLSS y FSR necesitan sus archivos en una carpeta.", "Desenha menor e aumenta. DLSS e FSR precisam dos arquivos deles numa pasta."],
  ["DLSS files folder", "Carpeta de archivos DLSS", "Pasta dos arquivos DLSS"],
  ["FSR files folder", "Carpeta de archivos FSR", "Pasta dos arquivos FSR"],
  ["Folder with nvngx_dlss.dll.", "Carpeta con nvngx_dlss.dll.", "Pasta com o nvngx_dlss.dll."],

  // ── Performance ──
  ["Memory limit, process priority and CPU compatibility.", "Límite de memoria, prioridad del proceso y compatibilidad de CPU.", "Limite de memória, prioridade do processo e compatibilidade de CPU."],
  ["Limit game memory", "Limitar memoria del juego", "Limitar memória do jogo"],
  ["Caps how much RAM the game may use so the rest of your PC stays responsive. Most games want 8 GB or more.", "Limita cuánta RAM puede usar el juego para que el resto del PC siga respondiendo. La mayoría quiere 8 GB o más.", "Limita quanta RAM o jogo pode usar para o resto do PC continuar respondendo. A maioria quer 8 GB ou mais."],
  ["Memory for the game", "Memoria para el juego", "Memória para o jogo"],
  ["Limit mode", "Modo de límite", "Modo do limite"],
  ["Soft — page out the rest", "Suave — el resto va al archivo de paginación", "Suave — o resto vai para a memória virtual"],
  ["Strict — close if exceeded", "Estricto — cerrar si se pasa", "Estrito — fechar se passar"],
  ["Soft keeps physical RAM under the limit and moves the rest to the page file: slower, but the game keeps running. Strict stops the game if it asks for more.", "Suave mantiene la RAM bajo el límite y manda el resto al archivo de paginación: más lento, pero el juego sigue. Estricto cierra el juego si pide más.", "Suave mantém a RAM abaixo do limite e manda o resto para a memória virtual: mais lento, mas o jogo continua. Estrito fecha o jogo se pedir mais."],
  ["Process priority", "Prioridad del proceso", "Prioridade do processo"],
  ["Normal", "Normal", "Normal"],
  ["Above normal", "Superior a lo normal", "Acima do normal"],
  ["High", "Alta", "Alta"],
  ["Automatic uses High on laptops and PCs with 16 GB of RAM or less, where the engine needs all the CPU it can get.", "Automático usa Alta en laptops y PCs con 16 GB de RAM o menos, donde el motor necesita toda la CPU posible.", "Automático usa Alta em notebooks e PCs com 16 GB de RAM ou menos, onde o motor precisa de toda a CPU possível."],
  ["CPU compatibility mode", "Modo de compatibilidad de CPU", "Modo de compatibilidade de CPU"],
  ["Turn on with Intel CPUs if a game closes right after starting.", "Actívalo con CPUs Intel si un juego se cierra justo al empezar.", "Ligue com CPUs Intel se um jogo fechar logo ao abrir."],
  ["Console timer rate", "Velocidad del reloj de la consola", "Velocidade do relógio do console"],
  ["Fixes games that run too fast. Turn off if a game becomes slow.", "Corrige juegos que van demasiado rápido. Apágalo si un juego se vuelve lento.", "Corrige jogos rápidos demais. Desligue se o jogo ficar lento."],
  ["Stack red-zone protection", "Protección de la zona roja de la pila", "Proteção da zona vermelha da pilha"],
  ["Prevents some crashes on Windows. Turn on only for games that crash.", "Evita algunos cierres en Windows. Actívalo solo en juegos que se cierran.", "Evita alguns travamentos no Windows. Ligue só em jogos que fecham."],

  // ── System ──
  ["What games see as the console user, language and network.", "Lo que los juegos ven como usuario, idioma y red de la consola.", "O que os jogos veem como usuário, idioma e rede do console."],
  ["User name", "Nombre de usuario", "Nome de usuário"],
  ["Name shown inside games.", "Nombre que aparece dentro de los juegos.", "Nome mostrado dentro dos jogos."],
  ["User ID", "ID de usuario", "ID do usuário"],
  ["Changing it gives games a fresh save profile.", "Cambiarlo da a los juegos un perfil de partidas nuevo.", "Mudar isso dá aos jogos um perfil de saves novo."],
  ["Console language", "Idioma de la consola", "Idioma do console"],
  ["Games use this to pick their language.", "Los juegos usan esto para elegir su idioma.", "Os jogos usam isso para escolher o idioma."],
  ["Network access", "Acceso a la red", "Acesso à rede"],
  ["Lets games use your local network. Some games hang with it on.", "Deja que los juegos usen tu red local. Algunos se cuelgan con esto encendido.", "Deixa os jogos usarem sua rede local. Alguns travam com isso ligado."],
  ["Emulate online services", "Emular servicios en línea", "Emular serviços online"],
  ["Compatibility shim for games that check online status.", "Ayuda a los juegos que comprueban si estás en línea.", "Ajuda jogos que verificam se você está online."],
  ["Achievement pop-ups", "Avisos de logros", "Avisos de conquistas"],
  ["Shows a notification when you unlock something.", "Muestra un aviso cuando desbloqueas algo.", "Mostra um aviso quando você desbloqueia algo."],
  ["Pop-up duration", "Duración del aviso", "Duração do aviso"],
  ["Seconds each notification stays on screen.", "Segundos que cada aviso queda en pantalla.", "Segundos que cada aviso fica na tela."],
  ["Skip startup logos", "Saltar logos de inicio", "Pular logos de início"],
  ["Skips logos and notices in games that allow it.", "Salta logos y avisos en los juegos que lo permiten.", "Pula logos e avisos nos jogos que permitem."],
  ["Game decides", "Lo decide el juego", "O jogo decide"],
  ["Let game decide", "Que decida el juego", "Deixar o jogo decidir"],
  ["Empty = the game decides.", "Vacío = lo decide el juego.", "Vazio = o jogo decide."],

  // ── Controller ──
  ["Gamepads work out of the box. Keyboard and mouse keys can be changed here.", "Los mandos funcionan sin configurar. Aquí puedes cambiar las teclas de teclado y ratón.", "Controles funcionam direto. Aqui você muda as teclas do teclado e do mouse."],
  ["Let Strata set the keys", "Dejar que Strata ponga las teclas", "Deixar o Strata definir as teclas"],
  ["Off = each engine keeps its own default keys. Your old files are kept as .bak.", "Apagado = cada motor usa sus teclas de siempre. Tus archivos anteriores quedan como .bak.", "Desligado = cada motor usa as teclas padrão dele. Seus arquivos antigos ficam como .bak."],
  ["Keyboard and mouse", "Teclado y ratón", "Teclado e mouse"],
  ["Reset keys", "Restablecer teclas", "Redefinir teclas"],
  ["Swap Cross and Circle", "Intercambiar Cruz y Círculo", "Trocar X e Círculo"],
  ["For controllers where you prefer the other confirm button.", "Para mandos donde prefieres el otro botón de aceptar.", "Para controles em que você prefere o outro botão de confirmar."],
  ["Stick dead zone", "Zona muerta del stick", "Zona morta do analógico"],
  ["How far a stick moves before the game notices.", "Cuánto se mueve el stick antes de que el juego lo note.", "Quanto o analógico se move antes de o jogo perceber."],
  ["Keyboard player", "Jugador con teclado", "Jogador no teclado"],
  ["Gamepads fill the other players in connection order.", "Los mandos ocupan los demás jugadores en orden de conexión.", "Os controles ocupam os outros jogadores na ordem de conexão."],
  ["Light bar color", "Color de la barra de luz", "Cor da barra de luz"],
  ["Controller speaker", "Altavoz del mando", "Alto-falante do controle"],
  ["50% is the normal level.", "50% es el nivel normal.", "50% é o nível normal."],
  ["Vibration", "Vibración", "Vibração"],
  ["Scales controller rumble.", "Ajusta la vibración del mando.", "Ajusta a vibração do controle."],
  ["Custom keys for this game", "Teclas propias para este juego", "Teclas próprias para este jogo"],
  ["This game has its own keys.", "Este juego tiene sus propias teclas.", "Este jogo tem teclas próprias."],
  ["This game uses the global keys.", "Este juego usa las teclas generales.", "Este jogo usa as teclas gerais."],
  ["Use global keys again", "Usar las teclas generales otra vez", "Usar as teclas gerais de novo"],
  ["Keys apply to AnyPS5. KytyPS5 and SharpEmu use their own fixed layout; the options below this list still apply to KytyPS5.", "Las teclas sirven para AnyPS5. KytyPS5 y SharpEmu usan su propio esquema fijo; las opciones debajo de esta lista sí sirven para KytyPS5.", "As teclas valem para o AnyPS5. O KytyPS5 e o SharpEmu usam um esquema fixo próprio; as opções abaixo desta lista ainda valem para o KytyPS5."],
  ["Player 1", "Jugador 1", "Jogador 1"],
  ["Player 2", "Jugador 2", "Jogador 2"],
  ["Player 3", "Jugador 3", "Jogador 3"],
  ["Player 4", "Jugador 4", "Jogador 4"],
  ["Mouse left", "Clic izquierdo", "Clique esquerdo"],
  ["Mouse right", "Clic derecho", "Clique direito"],
  ["Mouse middle", "Clic central", "Clique do meio"],
  ["Space", "Espacio", "Espaço"],
  ["Left Shift", "Mayús izq.", "Shift esq."],
  ["Right Shift", "Mayús der.", "Shift dir."],
  ["Left Ctrl", "Ctrl izq.", "Ctrl esq."],
  ["Right Ctrl", "Ctrl der.", "Ctrl dir."],
  ["Left Alt", "Alt izq.", "Alt esq."],
  ["Right Alt", "Alt der.", "Alt dir."],

  // ── Library settings ──
  ["Folders Strata watches for games. Each game is a folder with eboot.bin inside, or a .zar archive.", "Carpetas donde Strata busca juegos. Cada juego es una carpeta con eboot.bin dentro, o un archivo .zar.", "Pastas onde o Strata procura jogos. Cada jogo é uma pasta com eboot.bin dentro, ou um arquivo .zar."],
  ["Add folder", "Añadir carpeta", "Adicionar pasta"],
  ["Add a folder", "Añadir una carpeta", "Adicionar uma pasta"],
  ["Folder added.", "Carpeta añadida.", "Pasta adicionada."],
  ["Skipped folders", "Carpetas ignoradas", "Pastas ignoradas"],
  ["Install games to", "Instalar juegos en", "Instalar jogos em"],
  ["Automatic uses your own games folder (for example E:\\PS5Games) instead of Strata's folder on drive C.", "Automático usa tu propia carpeta de juegos (por ejemplo E:\\PS5Games) en vez de la carpeta de Strata en el disco C.", "Automático usa sua própria pasta de jogos (por exemplo E:\\PS5Games) em vez da pasta do Strata no disco C."],
  ["Change folder", "Cambiar carpeta", "Trocar pasta"],

  // ── Engines ──
  ["Strata runs games through open-source engines (all GPL-2.0), downloaded from their official GitHub releases. If one engine cannot run a game, Strata tries the next.", "Strata abre los juegos con motores de código abierto (todos GPL-2.0), descargados de sus versiones oficiales en GitHub. Si un motor no puede con un juego, Strata prueba el siguiente.", "O Strata abre os jogos com motores de código aberto (todos GPL-2.0), baixados das versões oficiais no GitHub. Se um motor não rodar o jogo, o Strata tenta o próximo."],
  ["PS5 engines", "Motores de PS5", "Motores de PS5"],
  ["Engines, best first", "Motores, el mejor primero", "Motores, o melhor primeiro"],
  ["Strata tries these from left to right. Use the arrows to change the order.", "Strata los prueba de izquierda a derecha. Usa las flechas para cambiar el orden.", "O Strata tenta da esquerda para a direita. Use as setas para mudar a ordem."],
  ["Try the next engine automatically", "Probar el siguiente motor automáticamente", "Tentar o próximo motor automaticamente"],
  ["If an engine closes or crashes right away, Strata starts the next one and writes everything to the log.", "Si un motor se cierra o falla al instante, Strata abre el siguiente y lo anota todo en el registro.", "Se um motor fechar ou travar logo de cara, o Strata abre o próximo e anota tudo no log."],
  ["Counts as failed before", "Cuenta como fallo antes de", "Conta como falha antes de"],
  ["An engine that closes sooner than this is treated as not working for that game.", "Un motor que se cierra antes de esto se toma como que no funciona con ese juego.", "Um motor que fecha antes disso é tratado como não funcionando nesse jogo."],
  ["Downloading… you can keep browsing.", "Descargando… puedes seguir navegando.", "Baixando… pode continuar navegando."],
  ["Nothing downloaded yet.", "Aún no hay nada descargado.", "Nada baixado ainda."],
  ["Engines folder", "Carpeta de motores", "Pasta dos motores"],
  ["Releases, saves and caches of every engine.", "Versiones, partidas y cachés de cada motor.", "Versões, saves e caches de cada motor."],
  ["Engine for this game", "Motor para este juego", "Motor para este jogo"],
  ["Automatic engine choice", "Elección automática del motor", "Escolha automática do motor"],
  ["Automatic tries the engines in order (Settings → Engines) and remembers the one that worked.", "Automático prueba los motores en orden (Ajustes → Motores) y recuerda el que funcionó.", "Automático tenta os motores em ordem (Configurações → Motores) e lembra o que funcionou."],
  ["Engine that worked", "Motor que funcionó", "Motor que funcionou"],
  ["Engine that ran the game", "Motor que abrió el juego", "Motor que rodou o jogo"],
  ["Forget", "Olvidar", "Esquecer"],
  ["Forget the engine that worked", "Olvidar el motor que funcionó", "Esquecer o motor que funcionou"],
  ["Next launch starts again from the first engine.", "La próxima vez empieza otra vez por el primer motor.", "Na próxima vez começa de novo pelo primeiro motor."],
  ["No engine for this game yet", "Aún no hay motor para este juego", "Ainda não há motor para este jogo"],
  ["Strata needs a PS5 engine to start games. Download KytyPS5, SharpEmu and AnyPS5 now?", "Strata necesita un motor de PS5 para abrir juegos. ¿Descargar KytyPS5, SharpEmu y AnyPS5 ahora?", "O Strata precisa de um motor de PS5 para abrir jogos. Baixar KytyPS5, SharpEmu e AnyPS5 agora?"],
  ["Downloads KytyPS5, SharpEmu and AnyPS5 from their official GitHub releases. If one cannot run a game, Strata tries the next.", "Descarga KytyPS5, SharpEmu y AnyPS5 de sus versiones oficiales en GitHub. Si uno no puede con un juego, Strata prueba el siguiente.", "Baixa KytyPS5, SharpEmu e AnyPS5 das versões oficiais no GitHub. Se um não rodar o jogo, o Strata tenta o próximo."],
  ["Get the rest", "Descargar los demás", "Baixar os outros"],
  ["Install the first", "Instala el primero", "Instale o primeiro"],

  // ── Community ──
  ["Share results with other players and show what you play. Everything here is off unless you turn it on.", "Comparte resultados con otros jugadores y muestra a qué juegas. Todo aquí está apagado hasta que lo enciendas.", "Compartilhe resultados com outros jogadores e mostre o que você joga. Tudo aqui fica desligado até você ligar."],
  ["Share test results", "Compartir resultados de pruebas", "Compartilhar resultados de testes"],
  ["After a game closes, send the game, engine, result, what the engine was missing and its error message (file paths and quoted text removed). No name, no logs. Working games appear on the website's Compatibility page; errors help fix the engines.", "Al cerrar un juego, envía el juego, el motor, el resultado, lo que le faltó al motor y su mensaje de error (sin rutas ni texto entre comillas). Sin nombre, sin registros. Los juegos que funcionan salen en la página de Compatibilidad de la web; los errores ayudan a mejorar los motores.", "Ao fechar um jogo, envia o jogo, o motor, o resultado, o que faltou no motor e a mensagem de erro dele (sem caminhos nem texto entre aspas). Sem nome, sem logs. Jogos que funcionam aparecem na página de Compatibilidade do site; os erros ajudam a melhorar os motores."],
  ["Use everyone's results", "Usar los resultados de todos", "Usar os resultados de todos"],
  ["Download the combined results (twice a day at most) so Strata starts games with the engine that worked for other players.", "Descarga los resultados de todos (máximo dos veces al día) para que Strata abra los juegos con el motor que les funcionó a otros.", "Baixa os resultados de todos (no máximo duas vezes por dia) para o Strata abrir os jogos com o motor que funcionou para outros jogadores."],
  ["Show on Discord", "Mostrar en Discord", "Mostrar no Discord"],
  ["Shows “Browsing the library” in the menus and the game with its engine while you play, on your Discord profile. Needs the Discord desktop app open on this PC.", "Muestra “Browsing the library” en los menús y el juego con su motor mientras juegas, en tu perfil de Discord. Necesita la app de Discord abierta en este PC.", "Mostra “Browsing the library” nos menus e o jogo com o motor enquanto você joga, no seu perfil do Discord. Precisa do app do Discord aberto neste PC."],
  ["What is sent, exactly", "Qué se envía exactamente", "O que é enviado, exatamente"],
  ["Help everyone find working games?", "¿Ayudar a todos a encontrar juegos que funcionan?", "Ajudar todo mundo a achar jogos que funcionam?"],
  ["Share results", "Compartir resultados", "Compartilhar resultados"],
  ["Thanks! Results will be shared.", "¡Gracias! Se compartirán los resultados.", "Valeu! Os resultados serão compartilhados."],
  ["Okay, nothing will be sent.", "Vale, no se enviará nada.", "Ok, nada será enviado."],
  ["Open the compatibility list", "Abrir la lista de compatibilidad", "Abrir a lista de compatibilidade"],

  // ── Backups ──
  ["Copies game saves (and, if you want, shader caches) into one file you can put back later.", "Copia las partidas (y si quieres, las cachés de shaders) en un archivo que puedes restaurar luego.", "Copia os saves (e, se quiser, os caches de shaders) num arquivo que você pode restaurar depois."],
  ["Back up all games", "Copiar todos los juegos", "Fazer backup de todos os jogos"],
  ["Back up now", "Copiar ahora", "Fazer backup agora"],
  ["Include shader caches", "Incluir cachés de shaders", "Incluir caches de shaders"],
  ["Bigger file, but games skip the shader stutter after a restore.", "Archivo más grande, pero los juegos no tienen tirones de shaders tras restaurar.", "Arquivo maior, mas os jogos não engasgam com shaders depois de restaurar."],
  ["Saves from every engine, in one file.", "Partidas de todos los motores, en un archivo.", "Saves de todos os motores, num arquivo."],
  ["Saves of this game from every engine.", "Partidas de este juego de todos los motores.", "Saves deste jogo de todos os motores."],
  ["Backups", "Copias", "Backups"],
  ["No backups yet.", "Aún no hay copias.", "Nenhum backup ainda."],
  ["Nothing saved yet.", "Aún no hay nada guardado.", "Nada salvo ainda."],
  ["Open backups folder", "Abrir carpeta de copias", "Abrir pasta de backups"],
  ["Restore this backup?", "¿Restaurar esta copia?", "Restaurar este backup?"],
  ["The saves in it replace the current ones. The current files are kept next to them as “.before-restore”.", "Sus partidas reemplazan las actuales. Los archivos actuales quedan al lado como “.before-restore”.", "Os saves dele substituem os atuais. Os arquivos atuais ficam ao lado como “.before-restore”."],

  // ── Appearance ──
  ["Language, colors, boot animation and motion.", "Idioma, colores, animación de inicio y movimiento.", "Idioma, cores, animação de início e movimento."],
  ["Language", "Idioma", "Idioma"],
  ["Language of the Strata interface. Engines and games keep their own language.", "Idioma de la interfaz de Strata. Los motores y juegos mantienen su propio idioma.", "Idioma da interface do Strata. Motores e jogos mantêm o idioma deles."],
  ["Check for updates", "Buscar actualizaciones", "Procurar atualizações"],
  ["Looks at Strata's GitHub releases at start and shows “Update” at the top when a new version is out.", "Al abrir, revisa las versiones de Strata en GitHub y muestra “Actualizar” arriba cuando sale una nueva.", "Ao abrir, olha as versões do Strata no GitHub e mostra “Atualizar” lá em cima quando sai uma nova."],
  ["Accent 1", "Color 1", "Cor 1"],
  ["Accent 2", "Color 2", "Cor 2"],
  ["Main buttons, progress and highlights.", "Botones principales, progreso y destacados.", "Botões principais, progresso e destaques."],
  ["Focus rings and secondary highlights.", "Bordes de foco y destacados secundarios.", "Bordas de foco e destaques secundários."],
  ["Boot animation", "Animación de inicio", "Animação de início"],
  ["Duo — your two accents", "Dúo — tus dos colores", "Duo — suas duas cores"],
  ["Spectrum — all six colors", "Espectro — los seis colores", "Espectro — as seis cores"],
  ["Shown every time Strata starts. Any key skips it.", "Se muestra cada vez que abre Strata. Cualquier tecla la salta.", "Aparece toda vez que o Strata abre. Qualquer tecla pula."],
  ["Preview the boot", "Ver la animación", "Ver a animação"],
  ["Plays the startup animation with the current colors.", "Reproduce la animación de inicio con los colores actuales.", "Mostra a animação de início com as cores atuais."],
  ["Reduce motion", "Reducir movimiento", "Reduzir movimento"],
  ["Shorter, calmer animations.", "Animaciones más cortas y tranquilas.", "Animações mais curtas e calmas."],
  ["Interface sounds", "Sonidos de la interfaz", "Sons da interface"],
  ["Soft clicks when moving around.", "Clics suaves al moverte.", "Cliques suaves ao navegar."],
  ["24-hour clock", "Reloj de 24 horas", "Relógio de 24 horas"],
  ["Bigger text and controls for playing on a TV with a controller. F9 turns it on and off.", "Texto y controles más grandes para jugar en una TV con mando. F9 lo enciende y apaga.", "Texto e controles maiores para jogar na TV com controle. F9 liga e desliga."],
  ["TV mode on (F9 to leave).", "Modo TV encendido (F9 para salir).", "Modo TV ligado (F9 para sair)."],
  ["TV mode off.", "Modo TV apagado.", "Modo TV desligado."],

  // ── Developer settings ──
  ["Logs, diagnostics and engine debug switches. Debug options slow games down a lot.", "Registros, diagnósticos y opciones de depuración del motor. Las opciones de depuración hacen los juegos mucho más lentos.", "Logs, diagnósticos e opções de depuração do motor. As opções de depuração deixam os jogos bem mais lentos."],
  ["Developer mode", "Modo desarrollador", "Modo desenvolvedor"],
  ["Adds the Developer tab with live logs, self-tests and diagnostics.", "Añade la pestaña Desarrollador con registros en vivo, autopruebas y diagnósticos.", "Adiciona a aba Desenvolvedor com logs ao vivo, autotestes e diagnósticos."],
  ["Log detail", "Detalle del registro", "Detalhe do log"],
  ["More detail helps when reporting a problem.", "Más detalle ayuda al reportar un problema.", "Mais detalhe ajuda ao reportar um problema."],
  ["Everything", "Todo", "Tudo"],
  ["Capture engine log", "Guardar el registro del motor", "Capturar o log do motor"],
  ["Shows the engine's own output in the console and saves it per session.", "Muestra la salida del motor en la consola y la guarda por sesión.", "Mostra a saída do motor no console e salva por sessão."],
  ["Session logs to keep", "Registros de sesión a guardar", "Logs de sessão a manter"],
  ["Older session logs are deleted automatically.", "Los registros más viejos se borran solos.", "Os logs mais antigos são apagados sozinhos."],
  ["Show engine command line", "Mostrar la línea de comandos del motor", "Mostrar a linha de comando do motor"],
  ["Displays the exact command used to start a game.", "Muestra el comando exacto usado para abrir el juego.", "Mostra o comando exato usado para abrir o jogo."],
  ["Logs · tests · diagnostics", "Registros · pruebas · diagnósticos", "Logs · testes · diagnósticos"],
  ["Run self-test", "Ejecutar autoprueba", "Rodar autoteste"],
  ["Running self-test…", "Ejecutando autoprueba…", "Rodando autoteste…"],
  ["Self-test", "Autoprueba", "Autoteste"],
  ["All checks passed.", "Todo bien.", "Tudo certo."],
  ["Press T or Run self-test.", "Pulsa T o Ejecutar autoprueba.", "Aperte T ou Rodar autoteste."],
  ["Export diagnostics", "Exportar diagnóstico", "Exportar diagnóstico"],
  ["Open logs folder", "Abrir carpeta de registros", "Abrir pasta de logs"],
  ["Open data folder", "Abrir carpeta de datos", "Abrir pasta de dados"],
  ["Log copied.", "Registro copiado.", "Log copiado."],
  ["Clipboard not available.", "Portapapeles no disponible.", "Área de transferência indisponível."],
  ["Filter text", "Filtrar texto", "Filtrar texto"],
  ["All sources", "Todas las fuentes", "Todas as fontes"],
  ["Following", "Siguiendo", "Acompanhando"],

  // ── per-game compatibility / dev ──
  ["What this game needs and how likely each engine is to run it.", "Qué necesita este juego y qué tan probable es que cada motor lo abra.", "O que este jogo precisa e qual a chance de cada motor rodar ele."],
  ["Logs and details for this game.", "Registros y detalles de este juego.", "Logs e detalhes deste jogo."],
  ["Reading the game's program…", "Leyendo el programa del juego…", "Lendo o programa do jogo…"],
  ["System functions used", "Funciones del sistema usadas", "Funções do sistema usadas"],
  ["System libraries", "Bibliotecas del sistema", "Bibliotecas do sistema"],
  ["Modules shipped with the game", "Módulos que trae el juego", "Módulos que vêm com o jogo"],
  ["High chance", "Alta probabilidad", "Chance alta"],
  ["Low chance", "Baja probabilidad", "Chance baixa"],
  ["Last tests", "Últimas pruebas", "Últimos testes"],
  ["Tests recorded", "Pruebas registradas", "Testes registrados"],
  ["Test engines again", "Probar los motores otra vez", "Testar os motores de novo"],
  ["Next launch tests the engines again.", "La próxima vez prueba los motores otra vez.", "Na próxima vez testa os motores de novo."],
  ["No engine has run it yet.", "Ningún motor lo ha abierto aún.", "Nenhum motor rodou ele ainda."],
  ["Open compatibility files", "Abrir archivos de compatibilidad", "Abrir arquivos de compatibilidade"],
  ["Latest engine log", "Último registro del motor", "Último log do motor"],
  ["Nothing yet. Play the game to see its log here.", "Aún nada. Juega para ver su registro aquí.", "Nada ainda. Jogue para ver o log aqui."],
  ["Open session logs", "Abrir registros de sesión", "Abrir logs de sessão"],
  ["Game folder", "Carpeta del juego", "Pasta do jogo"],

  // ── wizard / about ──
  ["Let's set up Strata.", "Vamos a configurar Strata.", "Vamos configurar o Strata."],
  ["Run setup again", "Repetir la configuración", "Refazer a configuração"],
  ["Only play games you dumped yourself from hardware you own.", "Juega solo juegos que tú mismo sacaste de una consola que es tuya.", "Jogue só jogos que você mesmo tirou de um console que é seu."],
  ["Only use games you own.", "Usa solo juegos que son tuyos.", "Use só jogos que são seus."],
  ["Support Strata", "Apoya a Strata", "Apoie o Strata"],
  ["Strata is free and has no ads. If it helps you, a small donation or a follow keeps it going.", "Strata es gratis y sin anuncios. Si te sirve, una pequeña donación o un follow ayudan a seguir.", "O Strata é grátis e sem anúncios. Se ajudar você, uma pequena doação ou um follow mantêm ele vivo."],
  ["Strata is not affiliated with, endorsed by or connected to any console maker. It ships no games, firmware or keys.", "Strata no tiene relación con ningún fabricante de consolas. No incluye juegos, firmware ni claves.", "O Strata não tem relação com nenhum fabricante de console. Não inclui jogos, firmware nem chaves."],
  ["This is a beta: expect rough edges. Developer → Export diagnostics creates a report you can share.", "Esto es una beta: puede haber fallos. Desarrollador → Exportar diagnóstico crea un informe para compartir.", "Isto é uma beta: pode ter falhas. Desenvolvedor → Exportar diagnóstico cria um relatório para compartilhar."],
  ["Not available in this build yet.", "Aún no está en esta versión.", "Ainda não está nesta versão."],
  ["Press any key to skip", "Pulsa cualquier tecla para saltar", "Aperte qualquer tecla para pular"],
  ["Experimental", "Experimental", "Experimental"],
  ["Fix", "Corrección", "Correção"],
  ["Debug", "Depuración", "Depuração"],
  ["the next engine", "el siguiente motor", "o próximo motor"],
  ["Strata can send an anonymous result after you play: the game, the engine, whether it worked, what the engine was missing and its error message (file paths removed). No name, no files, no logs. Games that work appear on strata.gbxstudios.com/compatibility. You can change this any time in Settings → Community.", "Strata puede enviar un resultado anónimo cuando juegas: el juego, el motor, si funcionó, qué le faltó al motor y su mensaje de error (sin rutas). Sin nombre, sin archivos, sin registros. Los juegos que funcionan salen en strata.gbxstudios.com/compatibility. Puedes cambiarlo cuando quieras en Ajustes → Comunidad.", "O Strata pode enviar um resultado anônimo depois que você joga: o jogo, o motor, se funcionou, o que faltou no motor e a mensagem de erro dele (sem caminhos). Sem nome, sem arquivos, sem logs. Jogos que funcionam aparecem em strata.gbxstudios.com/compatibility. Dá pra mudar quando quiser em Configurações → Comunidade."],
  ["Needs firmware", "Necesita firmware", "Precisa de firmware"],
  ["Less", "Menos", "Menos"],
  ["Too old", "Muy antigua", "Antiga demais"],
  ["May close game", "Puede cerrar el juego", "Pode fechar o jogo"],
  ["Title ID · platform", "ID · plataforma", "ID · plataforma"],
  ["Not found or unknown", "No encontrado o desconocido", "Não encontrado ou desconhecido"],
  ["Encrypted — not a decrypted dump", "Cifrado — no es un dump descifrado", "Criptografado — não é um dump descriptografado"],
  ["No KytyPS5 found in that folder.", "No se encontró KytyPS5 en esa carpeta.", "Nenhum KytyPS5 nessa pasta."],
  ["Open full console", "Abrir la consola completa", "Abrir o console completo"],
  ["PS5 games", "Juegos de PS5", "Jogos de PS5"],
  ["PS4 games", "Juegos de PS4", "Jogos de PS4"],
  ["This is a PS4 game", "Este es un juego de PS4", "Este é um jogo de PS4"],
  ["Extra content installed next to this game.", "Contenido extra instalado junto a este juego.", "Conteúdo extra instalado junto deste jogo."],
  ["Console", "Consola", "Console"],
  ["File", "Archivo", "Arquivo"],
  ["Silent", "Silencioso", "Silencioso"],
  ["Archive", "Archivo comprimido", "Arquivo compactado"],
];


// Lines that start with an English fixed part and end with something changing (a name, a number…).
const PREFIX = [
  ["Already installed: ", "Ya instalado: ", "Já instalado: "],
  ["Applied: ", "Aplicado: ", "Aplicado: "],
  ["Backup saved: ", "Copia guardada: ", "Backup salvo: "],
  ["Error: ", "Error: ", "Erro: "],
  ["Missing: ", "Falta: ", "Faltando: "],
  ["Played ", "Jugado ", "Jogado "],
  ["Ran with ", "Corrió con ", "Rodou com "],
  ["Saved ", "Guardado ", "Salvo "],
  ["Last ", "Último ", "Último "],
  ["Version ", "Versión ", "Versão "],
  ["Update ", "Actualizar ", "Atualizar "],
  ["Back up ", "Copiar ", "Backup de "],
  ["Your PC: ", "Tu PC: ", "Seu PC: "],
  ["Auto · last worked: ", "Auto · último que funcionó: ", "Auto · último que funcionou: "],
  ["The first launch reads the game and tests engines; after that Strata opens it straight in the engine that worked. The file for this game can be shared with friends: ", "La primera vez lee el juego y prueba motores; después Strata lo abre directo con el motor que funcionó. El archivo de este juego se puede compartir con amigos: ", "Na primeira vez ele lê o jogo e testa motores; depois o Strata abre direto no motor que funcionou. O arquivo deste jogo pode ser compartilhado com amigos: "],
];

// Whole lines with numbers inside.
const RX = [
  [/^(\d+) games found\.$/, "$1 juegos encontrados.", "$1 jogos encontrados."],
  [/^(\S+) skipped — trying (.+)…$/, "$1 saltado — probando $2…", "$1 pulado — tentando $2…"],
  [/^(\S+) closed after (\d+) s — trying (.+)…$/, "$1 se cerró a los $2 s — probando $3…", "$1 fechou após $2 s — tentando $3…"],
  [/^(.+) · worked after (\d+) s$/, "$1 · funcionó tras $2 s", "$1 · funcionou após $2 s"],
  [/^(.+) · failed after (\d+) s$/, "$1 · falló tras $2 s", "$1 · falhou após $2 s"],
  [/^(\d+) found\.$/, "$1 encontrados.", "$1 encontrados."],
  [/^(\d+) in library$/, "$1 en la biblioteca", "$1 na biblioteca"],
  [/^Applied (.+)\.$/, "Aplicado: $1.", "Aplicado: $1."],
  [/^Library · (\d+) games?$/, "Biblioteca · $1 juegos", "Biblioteca · $1 jogos"],
  [/^(\d+) of (\d+) tests worked$/, "$1 de $2 pruebas funcionaron", "$1 de $2 testes funcionaram"],
  [/^(\d+) missing function\(s\)$/, "Faltan $1 funciones", "Faltam $1 funções"],
  [/^Your PC has (\d+) GB\. Suggested: (\d+) GB\.$/, "Tu PC tiene $1 GB. Sugerido: $2 GB.", "Seu PC tem $1 GB. Sugerido: $2 GB."],
  [/^Strata (.+) is out\. Click "Update" at the top\.$/, "Salió Strata $1. Pulsa \"Actualizar\" arriba.", "Saiu o Strata $1. Clique em \"Atualizar\" lá em cima."],
  [/^Auto \((\d+) Hz on this monitor\)$/, "Auto ($1 Hz en este monitor)", "Auto ($1 Hz neste monitor)"],
  [/^Found (\d+) \.pkg files in your folders\.$/, "Hay $1 archivos .pkg en tus carpetas.", "Tem $1 arquivos .pkg nas suas pastas."],
  [/^(\d+) min ago$/, "hace $1 min", "há $1 min"],
  [/^(\d+) h ago$/, "hace $1 h", "há $1 h"],
  [/^(\d+) days ago$/, "hace $1 días", "há $1 dias"],
  [/^(.+) ready\. The others can be added later\.$/, "$1 listo. Los demás se pueden añadir luego.", "$1 pronto. Os outros podem ser adicionados depois."],
  [/^(.+) — tuned settings ready\.$/, "$1 — ajustes listos.", "$1 — configurações prontas."],
  [/^(Three|Four) quick steps\. Everything can be changed later in Settings\.$/, "Unos pasos rápidos. Todo se puede cambiar luego en Ajustes.", "Alguns passos rápidos. Tudo pode ser mudado depois em Configurações."],
];

const LANGS = { es: 1, pt: 2 };
let col = 0;
const EXACT = new Map();
// Game titles are never translated, even when one matches a UI word ("Control", "Home"…).
const KEEP = new Set();

export function protect(titles) {
  for (const t of titles || []) if (t) KEEP.add(String(t).trim());
}

export function language() {
  return col === 1 ? "es" : col === 2 ? "pt" : "en";
}

/** "auto" follows the Windows/browser language. */
export function resolve(setting) {
  if (setting && setting !== "auto") return LANGS[setting] ? setting : "en";
  const n = (navigator.languages?.[0] || navigator.language || "en").toLowerCase();
  return n.startsWith("es") ? "es" : n.startsWith("pt") ? "pt" : "en";
}

export function tr(text) {
  if (!col || !text) return text;
  const core = text.trim();
  if (!core || KEEP.has(core)) return text;
  const lead = text.slice(0, text.indexOf(core[0]));
  const trail = text.slice(lead.length + core.length);
  const hit = EXACT.get(core);
  if (hit) return lead + hit + trail;
  for (const [rx, ...out] of RX) if (rx.test(core)) return lead + core.replace(rx, out[col - 1]) + trail;
  for (const p of PREFIX) {
    if (core.startsWith(p[0]) && core.length > p[0].length) return lead + p[col] + core.slice(p[0].length) + trail;
    if (core === p[0].trim()) return lead + p[col].trim() + trail;
  }
  return text;
}

// Log lines are the engines' own words: never translated.
const SKIP = "script, style, textarea, .log, .console, .cmd, code, #loglines, [data-notr]";
const ATTRS = ["placeholder", "title", "aria-label"];

function translateNode(n) {
  if (n.nodeType === 3) {
    const p = n.parentElement;
    if (!p || p.closest(SKIP)) return;
    const t = tr(n.nodeValue);
    if (t !== n.nodeValue) n.nodeValue = t;
    return;
  }
  if (n.nodeType !== 1 || n.closest(SKIP)) return;
  for (const a of ATTRS) {
    const v = n.getAttribute(a);
    if (v) { const t = tr(v); if (t !== v) n.setAttribute(a, t); }
  }
  const w = document.createTreeWalker(n, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  for (let c = w.nextNode(); c; c = w.nextNode()) {
    if (c.nodeType === 1) {
      if (c.matches(SKIP)) continue;
      for (const a of ATTRS) {
        const v = c.getAttribute(a);
        if (v) { const t = tr(v); if (t !== v) c.setAttribute(a, t); }
      }
    } else translateNode(c);
  }
}

/** Start translating everything drawn from now on. Returns the language in use. */
export function start(setting) {
  const lang = resolve(setting);
  col = LANGS[lang] || 0;
  document.documentElement.lang = lang;
  if (!col) return lang;
  EXACT.clear();
  for (const row of T) EXACT.set(row[0], row[col]);
  translateNode(document.body);
  new MutationObserver((list) => {
    for (const m of list) {
      if (m.type === "characterData") translateNode(m.target);
      else if (m.type === "attributes") translateNode(m.target);
      else m.addedNodes.forEach(translateNode);
    }
  }).observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: ATTRS });
  return lang;
}

export const _test = { T, PREFIX, RX };
