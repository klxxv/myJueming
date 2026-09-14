const rows = {
  agentBrowserUnavailable: ['浏览器演示预览不提供 MCP 或嵌入式 Agent', 'MCP and the embedded agent are unavailable in the browser demo', 'ブラウザーのデモでは MCP と組み込みエージェントを利用できません', 'MCP et l’agent intégré ne sont pas disponibles dans la démo du navigateur', 'MCP und der integrierte Agent sind in der Browser-Demo nicht verfügbar'],
  companionGarden: ['决明小花园', 'Jueming garden', 'Jueming の庭', 'Jardin Jueming', 'Jueming-Garten'],
  companionPlant: ['决明草与花朵', 'Cassia plants and flowers', 'エビスグサと花', 'Cassia et fleurs', 'Kassia und Blüten'],
  companionPetGardenCat: ['摸摸花园里的猫猫', 'Pet the garden cat', '庭の猫をなでる', 'Caresser le chat du jardin', 'Die Gartenkatze streicheln'],
  companionPetGardenDog: ['摸摸花园里的狗狗', 'Pet the garden dog', '庭の犬をなでる', 'Caresser le chien du jardin', 'Den Gartenhund streicheln'],
  companionCat: ['橘猫', 'Ginger cat', '茶トラ猫', 'Chat roux', 'Rote Katze'],
  companionDog: ['金毛狗狗', 'Golden retriever', 'ゴールデンレトリバー', 'Golden retriever', 'Golden Retriever'],
  companionButterfly: ['引导工作的蝴蝶', 'Butterfly guiding your work', '作業を案内する蝶', 'Papillon qui guide votre travail', 'Schmetterling als Arbeitshilfe'],
  companionReview: ['需要你审核这次修改', 'This change needs your review', 'この変更を確認してください', 'Cette modification attend votre validation', 'Diese Änderung wartet auf Ihre Prüfung'],
  companionWorking: ['助手正在工作', 'The assistant is working', 'アシスタントが作業中です', 'L’assistant travaille', 'Der Assistent arbeitet'],
  companionOpenAgent: ['查看助手', 'View assistant', 'アシスタントを表示', 'Voir l’assistant', 'Assistenten anzeigen'],
  companionSync: ['应用操作与当前工程同步', 'App actions stay in sync with the current project', 'アプリの操作は現在のプロジェクトと同期されます', 'Les actions de l’application restent synchronisées avec le projet actuel', 'App-Aktionen bleiben mit dem aktuellen Projekt synchron'],
  companionRest: ['猫猫、狗狗和决明草都在这里', 'The cat, dog and cassia are all here', '猫も犬もエビスグサもここにいます', 'Le chat, le chien et le cassia sont tous ici', 'Katze, Hund und Kassia sind alle hier'],
  companionHome: ['猫猫与狗狗的小家', 'A home for the cat and dog', '猫と犬のおうち', 'La maison du chat et du chien', 'Das Zuhause von Katze und Hund'],
  companionPetCat: ['摸摸猫猫', 'Pet the cat', '猫をなでる', 'Caresser le chat', 'Die Katze streicheln'],
  companionPetDog: ['摸摸狗狗', 'Pet the dog', '犬をなでる', 'Caresser le chien', 'Den Hund streicheln'],
  companionCatBed: ['趴在猫窝里的橘猫', 'Ginger cat resting in its bed', 'ベッドでくつろぐ茶トラ猫', 'Chat roux dans son panier', 'Rote Katze in ihrem Körbchen'],
  companionDogHouse: ['在小屋前休息的金毛', 'Golden retriever resting by its house', '小屋の前で休むゴールデンレトリバー', 'Golden retriever devant sa niche', 'Golden Retriever vor seiner Hütte'],
  companionInGarden: ['去小花园陪你工作了', 'In the garden, keeping you company', '庭で作業を見守っています', 'Au jardin pour vous tenir compagnie', 'Im Garten, um Ihnen Gesellschaft zu leisten'],
  companionCatNotice: ['猫猫伸了个懒腰', 'The cat stretches', '猫が伸びをしました', 'Le chat s’étire', 'Die Katze streckt sich'],
  companionDogNotice: ['狗狗向你摇摇尾巴', 'The dog wags its tail at you', '犬がしっぽを振っています', 'Le chien remue la queue pour vous', 'Der Hund wedelt Ihnen zu'],
} as const;
type Key = keyof typeof rows;
const catalogue = (index: 0 | 1 | 2 | 3 | 4) => Object.fromEntries(Object.entries(rows).map(([key, values]) => [key, values[index]])) as Record<Key, string>;
export const companionMessages = { zh: catalogue(0), en: catalogue(1), ja: catalogue(2), fr: catalogue(3), de: catalogue(4) };
