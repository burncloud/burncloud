use std::collections::{BTreeMap, BTreeSet};
use dioxus::prelude::*;
use crate::{app::Route, backend::{Channel, ChannelService}, components::Icon};

fn status(channel:&Channel)->&'static str { if channel.status==1 {"Active"} else {"Down"} }

#[component]
pub fn Providers()->Element {
    let mut resource=use_resource(move||async move{ChannelService::list(100).await});
    let mut edit=use_signal(||None::<Channel>);
    let mut busy=use_signal(||false);
    let mut notice=use_signal(String::new);
    let mut error=use_signal(String::new);
    let mut name=use_signal(String::new); let mut type_id=use_signal(||1i32); let mut key=use_signal(String::new);
    let mut base_url=use_signal(String::new); let mut models=use_signal(String::new); let mut group=use_signal(||"default".to_string());
    let mut weight=use_signal(||100i32); let mut priority=use_signal(||0i64); let mut rpm=use_signal(String::new); let mut tpm=use_signal(String::new);
    let snapshot=resource.read().clone(); let load_error=snapshot.as_ref().and_then(|r|r.as_ref().err().cloned()); let list=snapshot.and_then(Result::ok).unwrap_or_default();
    let total=list.len(); let active=list.iter().filter(|c|c.status==1).count();
    rsx!{
        div{class:"page",
            div{class:"page-header",div{h2{class:"page-title","Providers"}p{class:"page-subtitle","Real Channel configuration: credentials, models, group, priority, weight and capacity limits."}}div{class:"header-actions",button{class:"button button-secondary",onclick:move |_|resource.restart(),"Refresh"}button{class:"button button-primary",onclick:move |_|{edit.set(Some(Channel::default()));name.set(String::new());type_id.set(1);key.set(String::new());base_url.set(String::new());models.set(String::new());group.set("default".to_string());weight.set(100);priority.set(0);rpm.set(String::new());tpm.set(String::new());error.set(String::new());},Icon{name:"plus"}"Add Provider"}}}
            div{class:"metrics",div{class:"card metric",div{class:"metric-copy",span{class:"metric-label","Providers"}span{class:"metric-value","{total}"}}}div{class:"card metric",div{class:"metric-copy",span{class:"metric-label","Active"}span{class:"metric-value","{active}"}}}}
            if !notice().is_empty(){div{class:"terminal auth-status","{notice}"}} if !error().is_empty(){div{class:"terminal auth-status auth-status-error","{error}"}}
            if let Some(message)=load_error{div{class:"terminal auth-status auth-status-error","{message}"}}else{
                div{class:"card table-card",if list.is_empty(){div{class:"card-pad small muted","No provider channels configured."}}else{div{class:"table-wrap",table{class:"data-table",thead{tr{th{"Provider"}th{"Type"}th{"Models"}th{"Group"}th{class:"right","Priority"}th{class:"right","Weight"}th{"Capacity"}th{"Status"}th{"Actions"}}}tbody{
                    for channel in list { {
                        let channel_edit=channel.clone(); let id_delete=channel.id; let base=channel.base_url.clone().unwrap_or_else(||"default base URL".to_string()); let cap=format!("RPM {} • TPM {}",channel.rpm_cap.map(|v|v.to_string()).unwrap_or_else(||"∞".to_string()),channel.tpm_cap.map(|v|v.to_string()).unwrap_or_else(||"∞".to_string())); let s=status(&channel);
                        rsx!{tr{key:"{channel.id}",td{div{class:"two-line",span{class:"table-primary","{channel.name}"}small{class:"mono","{base}"}}}td{class:"mono","{channel.type_}"}td{class:"mono muted","{channel.models}"}td{"{channel.group}"}td{class:"right","{channel.priority}"}td{class:"right","{channel.weight}"}td{class:"small muted","{cap}"}td{span{class:if channel.status==1{"badge badge-success"}else{"badge badge-error"},"{s}"}}td{div{class:"row gap-2",button{class:"button button-ghost button-sm",onclick:move |_|{name.set(channel_edit.name.clone());type_id.set(channel_edit.type_);key.set(channel_edit.key.clone());base_url.set(channel_edit.base_url.clone().unwrap_or_default());models.set(channel_edit.models.clone());group.set(channel_edit.group.clone());weight.set(channel_edit.weight);priority.set(channel_edit.priority);rpm.set(channel_edit.rpm_cap.map(|v|v.to_string()).unwrap_or_default());tpm.set(channel_edit.tpm_cap.map(|v|v.to_string()).unwrap_or_default());edit.set(Some(channel_edit.clone()));},"Edit"}button{class:"button button-ghost button-sm danger",disabled:busy(),onclick:move |_|{busy.set(true);error.set(String::new());spawn(async move{match ChannelService::delete(id_delete).await{Ok(())=>{notice.set("Provider deleted.".to_string());resource.restart();},Err(e)=>error.set(format!("Delete failed: {e}"))}busy.set(false);});},"Delete"}}}}}
                    } }
                }}}}}
            }
            if let Some(current)=edit(){
                div{class:"drawer-backdrop",onclick:move |_|edit.set(None)}aside{class:"drawer",div{class:"drawer-head",h2{if current.id>0{"Edit Provider"}else{"Add Provider"}}button{class:"close-button",onclick:move |_|edit.set(None),"×"}}div{class:"drawer-body stack-lg",
                    div{class:"grid-2",div{class:"field",label{"Name"}input{class:"input",value:"{name}",oninput:move|e|name.set(e.value())}}div{class:"field",label{"Provider Type ID"}input{class:"input",r#type:"number",value:"{type_id}",oninput:move|e|type_id.set(e.value().parse().unwrap_or(1))}}}
                    div{class:"field",label{"API Key / Credential"}input{class:"input mono",r#type:"password",value:"{key}",oninput:move|e|key.set(e.value())}}
                    div{class:"field",label{"Base URL"}input{class:"input mono",value:"{base_url}",oninput:move|e|base_url.set(e.value())}}
                    div{class:"field",label{"Models (comma-separated)"}textarea{class:"textarea mono",rows:"4",value:"{models}",oninput:move|e|models.set(e.value())}}
                    div{class:"grid-2",div{class:"field",label{"Group"}input{class:"input",value:"{group}",oninput:move|e|group.set(e.value())}}div{class:"field",label{"Weight"}input{class:"input",r#type:"number",value:"{weight}",oninput:move|e|weight.set(e.value().parse().unwrap_or(100))}}div{class:"field",label{"Priority"}input{class:"input",r#type:"number",value:"{priority}",oninput:move|e|priority.set(e.value().parse().unwrap_or(0))}}div{class:"field",label{"RPM Cap"}input{class:"input",r#type:"number",value:"{rpm}",oninput:move|e|rpm.set(e.value())}}div{class:"field",label{"TPM Cap"}input{class:"input",r#type:"number",value:"{tpm}",oninput:move|e|tpm.set(e.value())}}}
                    button{class:"button button-primary",disabled:busy(),onclick:move |_|{let item=Channel{id:current.id,type_:type_id(),key:key(),name:name().trim().to_string(),base_url:if base_url().trim().is_empty(){None}else{Some(base_url().trim().to_string())},models:models().trim().to_string(),group:group().trim().to_string(),status:if current.id>0{current.status}else{1},weight:weight(),priority:priority(),param_override:current.param_override.clone(),header_override:current.header_override.clone(),api_version:current.api_version.clone(),model_mapping:current.model_mapping.clone(),rpm_cap:rpm().trim().parse().ok(),tpm_cap:tpm().trim().parse().ok()};if item.name.is_empty()||item.key.is_empty()||item.models.is_empty(){error.set("Name, credential and models are required.".to_string());return;}let updating=item.id>0;busy.set(true);error.set(String::new());spawn(async move{let r=if updating{ChannelService::update(&item).await}else{ChannelService::create(&item).await};match r{Ok(())=>{notice.set(if updating{"Provider updated.".to_string()}else{"Provider created.".to_string()});edit.set(None);resource.restart();},Err(e)=>error.set(format!("Save failed: {e}"))}busy.set(false);});},if busy(){"Saving…"}else{"Save Provider"}}
                }}
            }
        }
    }
}

#[component]
pub fn Models()->Element{
    let mut resource=use_resource(move||async move{ChannelService::list(100).await});let snapshot=resource.read().clone();let error=snapshot.as_ref().and_then(|r|r.as_ref().err().cloned());let channels=snapshot.and_then(Result::ok).unwrap_or_default();let empty=channels.is_empty();let mut map:BTreeMap<String,(usize,usize,BTreeSet<String>)>=BTreeMap::new();for c in &channels{for m in c.models.split(',').map(str::trim).filter(|m|!m.is_empty()){let e=map.entry(m.to_string()).or_insert((0,0,BTreeSet::new()));e.0+=1;if c.status==1{e.1+=1;}e.2.insert(c.name.clone());}}
    rsx!{div{class:"page",div{class:"page-header",div{h2{class:"page-title","Models"}p{class:"page-subtitle","Derived from real Channel.models. There is no separate model CRUD API in the server."}}button{class:"button button-secondary",onclick:move |_|resource.restart(),"Refresh"}}if let Some(message)=error{div{class:"terminal auth-status auth-status-error","{message}"}}else{div{class:"card table-card",if empty{div{class:"card-pad small muted","No channels configured."}}else{div{class:"table-wrap",table{class:"data-table",thead{tr{th{"Model"}th{class:"right","Providers"}th{class:"right","Active"}th{"Source Channels"}th{"Management"}}}tbody{for(model,(providers,active,sources))in map{{let source=sources.into_iter().collect::<Vec<_>>().join(", ");rsx!{tr{key:"{model}",td{class:"table-primary mono","{model}"}td{class:"right","{providers}"}td{class:"right","{active}"}td{"{source}"}td{Link{class:"button button-ghost button-sm",to:Route::Providers{},"Edit Providers"}}}}}}}}}}}}
}

#[component]
pub fn Routes()->Element{
    let mut resource=use_resource(move||async move{ChannelService::list(100).await});let snapshot=resource.read().clone();let error=snapshot.as_ref().and_then(|r|r.as_ref().err().cloned());let channels=snapshot.and_then(Result::ok).unwrap_or_default();let mut groups:BTreeMap<String,Vec<Channel>>=BTreeMap::new();for c in channels{groups.entry(c.group.clone()).or_default().push(c);}for rows in groups.values_mut(){rows.sort_by_key(|c|(c.priority,-c.weight));}
    rsx!{div{class:"page",div{class:"page-header",div{h2{class:"page-title","Routes"}p{class:"page-subtitle","Derived from Channel.group, priority and weight — the routing configuration that actually exists."}}button{class:"button button-secondary",onclick:move |_|resource.restart(),"Refresh"}}if let Some(message)=error{div{class:"terminal auth-status auth-status-error","{message}"}}else if groups.is_empty(){div{class:"card card-pad small muted","No routing groups derived yet."}}else{div{class:"stack-lg",for(group,rows)in groups{div{class:"card card-pad stack",div{class:"row between",h3{"{group}"}Link{class:"button button-secondary button-sm",to:Route::Providers{},"Manage Channels"}}div{class:"table-wrap",table{class:"data-table",thead{tr{th{"Order"}th{"Provider"}th{"Models"}th{class:"right","Priority"}th{class:"right","Weight"}th{"Status"}}}tbody{for(index,c)in rows.iter().enumerate(){{let order=index+1;let s=status(c);rsx!{tr{key:"{c.id}",td{class:"mono","#{order}"}td{class:"table-primary","{c.name}"}td{class:"mono muted","{c.models}"}td{class:"right","{c.priority}"}td{class:"right","{c.weight}"}td{span{class:if c.status==1{"badge badge-success"}else{"badge badge-error"},"{s}"}}}}}}}}}}}}}}
}
