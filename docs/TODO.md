# service-models 增加 [hugginface.co](https://huggingface.co/api/models) 对接
{
    "work": "https://huggingface.co/api/models是hugginface的查询",
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}

# service-ip 判断用户当前处于哪个网络环境
{
    "work": "编写crates/ip，项目名为burncloud-ip，用来确认是中国用户还是国际用户，如果是中国用户则返回CN，如果是海外用户则返回WORLD。现在有两个接口可以判断位置，访问ip api:http://ip-api.com/json/ 他的返回内容（{"status":"success","country":"China","countryCode":"CN","region":"GD","regionName":"Guangdong","city":"Guangzhou","zip":"","lat":23.1181,"lon":113.2539,"timezone":"Asia/Shanghai","isp":"Chinanet","org":"Chinanet GD","as":"AS4134 CHINANET-BACKBONE","query":"219.137.108.46"}）,如果访问不到则换成https://ipinfo.io/，他的返回内容：{"ip": "119.237.241.196","hostname": "n119237241196.netvigator.com","city": "Hong Kong","region": "Hong Kong","country": "HK","loc": "22.2783,114.1747","org": "AS4760 HKT Limited","postal": "999077","timezone": "Asia/Hong_Kong","readme": "https://ipinfo.io/missingauth"}",
    "rules": [
        "使用中文回复",
        "使用最精简的代码来编写",
        "只允许使用rust编写代码"
    ]
}

# client-models页面
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