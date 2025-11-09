# service-models 删除模型
- 删除模型需要清理对应的文件，包含清理.1,.2,.3的后缀
- 例Qwen2.5-7B-Instruct-GGUF.1，Qwen2.5-7B-Instruct-GGUF.2，Qwen2.5-7B-Instruct-GGUF.3。
- 直接先去掉文件Qwen2.5-7B-Instruct-GGUF.guff的后缀，再删除Qwen2.5-7B-Instruct-GGUF*文件


# service-models 下载功能
{
    "work": "client-models现在增加模型的下载功能实现，当用户点击下载模型，先读取service-setting name=dir_data存放位置，如果没有读取到值，则设定value=./data，接受用户传入id和path，例：{host}api/models/{id}/resolve/main/{path}?download=true，host=get_huggingface_host,id=Qwen/Qwen2.5-7B-Instruct-GGUF,path=qwen2.5-7b-instruct-fp16-00001-of-00004.gguf，下载使用burncloud-download来下载",
    "depend": [
        "crates/service-setting",
        "crates/service-models",
        "crates/download"
    ],
    "rules": ["中文回复","代码最精简化","rust编程"]
}

# [complete] service-models 列出所有GUFF文件
{
    "work": "get_model_files获取所有文件，编写代码从中列出所有GUFF文件",
    "rules": ["使用中文回复","使用最精简的代码来编写","只允许使用rust编写代码"]
}

# [complete] service-models 递归获取所有文件
{
    "work": "service-models 用get_huggingface_host获取host，再根据传入的id(例：deepseek-ai/DeepSeek-OCR)，组成url: {host}api/models/{id}/tree/main 获取文件列表，如果获取的列表内容里面包含文件夹，则需要继续遍历。示例json:[{"type":"directory","oid":"e0e4421f3463b5c1f8db4514c1b39ee191da6076","size":0,"path":".ipynb_checkpoints"},{"type":"directory","oid":"8ab42b51620a04aa257174201e49eca7885265a3","size":0,"path":"assets"},{"type":"file","oid":"12296852e0362bac092cd2a53bb676d7af4023be","size":1783,"path":".gitattributes"},{"type":"file","oid":"d62e3bef9f054f21b7fc616365850fbf879a99ff","size":1084,"path":"LICENSE"}], 里面directory的assets，则需要组成url: {host}api/models/{id}/tree/main/assets 获取assets的文件列表，如果里面还包含文件夹，则要继续遍历。当获取了所有文件列表，把这些数据以二维数据的方式存在rust列表中并返回结果",
    "rules": ["使用中文回复","使用最精简的代码来编写","只允许使用rust编写代码"]
}

# [complete] service-models 设置url host
{
    "work": "service-models读取service-setting name=huggingface 如果 value不存在则读取service-ip 的 get_location,如果是CN则 service-setting name=hugginface value=https://huggingface.co/，其它则 service-setting name=hugginface value=https://hf-mirror.com/",
    "rules": ["使用中文回复","使用最精简的代码来编写","只允许使用rust编写代码"]
}

# [complete] client-models 弹出功能修改
{
    "work": "client-models现在增加模型的弹出窗口，没有正确的弹出一层，而是直接在菜单栏上面直接下拉了下来，而且加载hugginface数据的时候并没有加载提示，请修改上面的提的问题",
    "depend": [
        "crates/service-setting",
        "crates/service-models",
        "crates/download"
    ],
    "rules": ["使用中文回复","使用最精简的代码来编写","只允许使用rust编写代码"]
}


# [complete] client-models 添加模型页面
{
    "work": "在client-models页面，添加模型点击之后打开弹出页面，页面调用service-models fetch_from_huggingfaced 加载搜索结果，通过搜索结果，搜索结果按每行一条列出来，在每一行增加下载按钮，具体下载功能暂时不做，只实现点击下载功能把这一条数据导入到本地模型里面。",
    "depend": [
        "crates/service-models"
    ],
    "rules": ["使用中文回复","使用最精简的代码来编写","只允许使用rust编写代码"]
}


# [complete] service-models 增加 https://huggingface.co/api/models 对接
{
    "work": "读取service-ip的值,world使用 https://huggingface.co/api/models查询数据，cn使用https://hf-mirror.com/api/models数据，他们返回的格式都是统一如下：[{"_id":"6909b87b734dd24dd19da3ef","id":"moonshotai/Kimi-K2-Thinking","likes":578,"trendingScore":580,"private":false,"downloads":5119,"tags":["transformers","safetensors","kimi_k2","text-generation","conversational","custom_code","license:other","autotrain_compatible","endpoints_compatible","compressed-tensors","region:us"],"pipeline_tag":"text-generation","library_name":"transformers","createdAt":"2025-11-04T08:25:31.000Z","modelId":"moonshotai/Kimi-K2-Thinking"},{"_id":"68f8dfe68cb208be9702aa87","id":"MiniMaxAI/MiniMax-M2","likes":1178,"trendingScore":319,"private":false,"downloads":846249,"tags":["transformers","safetensors","minimax_m2","text-generation","conversational","custom_code","arxiv:2504.07164","arxiv:2509.06501","arxiv:2509.13160","license:mit","autotrain_compatible","fp8","region:us"],"pipeline_tag":"text-generation","library_name":"transformers","createdAt":"2025-10-22T13:45:10.000Z","modelId":"MiniMaxAI/MiniMax-M2"}]",
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}

# [complete] service-ip cache 读取
{
    "work": "被其它程序调用，先读取burncloud-service-setting的name=location，存在值，则直接返回字符串，如果不存在，则继续查询当前地区是CN还是WORLD。则写入burncloud-service-setting的name=location value=CN|WORLD",
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}


# [complete] service-setting 初始化
{
    "work": "编写crates/service/crates/service-setting 命名项目为 burncloud-service-setting，此项目为服务类，当burncloud-service-setting被其它项目调用的时候通过输入name获取value值 。name为主键。编写增加，修改，删除功能。",
    "depend": [
        "crates/database/crates/database-setting"
    ],
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}

# [complete] database-setting 
{
    "work": "编写crates/database/crates/database-setting 命名项目为 burncloud-database-setting，此项目主要是创建数据表setting，字段有name 和 value，当burncloud-databse-setting被其它项目调用的时候通过输入name获取value值 。name为主键。编写增加，修改，删除功能。",
    "depend": [
        "crates/database"
    ],
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码",
        "不要重写sqlx,请依赖 burncloud-database完成数据写入"
    ]
}

# [complete] service-ip 判断用户当前处于哪个网络环境
{
    "work": "编写crates/ip，项目名为burncloud-ip，用来确认是中国用户还是国际用户，如果是中国用户则返回CN，如果是海外用户则返回WORLD。现在有两个接口可以判断位置，访问ip api:http://ip-api.com/json/ 他的返回内容（{"status":"success","country":"China","countryCode":"CN","region":"GD","regionName":"Guangdong","city":"Guangzhou","zip":"","lat":23.1181,"lon":113.2539,"timezone":"Asia/Shanghai","isp":"Chinanet","org":"Chinanet GD","as":"AS4134 CHINANET-BACKBONE","query":"219.137.108.46"}）,如果访问不到则换成https://ipinfo.io/，他的返回内容：{"ip": "119.237.241.196","hostname": "n119237241196.netvigator.com","city": "Hong Kong","region": "Hong Kong","country": "HK","loc": "22.2783,114.1747","org": "AS4760 HKT Limited","postal": "999077","timezone": "Asia/Hong_Kong","readme": "https://ipinfo.io/missingauth"}",
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}

# [complete] client-models页面
{
    "work": "编写crates/client-models, crate名字是burncloud-client-models，编写大模型管理页面，具体的字段参数crates/service-models里面的代码。",
    "depend": [
        "crates/client-shared",
        "crates/service-models"
    ],
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}