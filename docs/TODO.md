# client-models 增加功能
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


# service-models 增加 https://huggingface.co/api/models 对接
{
    "work": "读取service-ip的值,world使用 https://huggingface.co/api/models查询数据，cn使用https://hf-mirror.com/api/models数据",
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