export async function onload(api) {
  api.ui.registerSettings({id:'preferences',title:'网络示例设置',fields:[{key:'show_status',label:'记录响应状态',type:'boolean',default:true}]});
  api.ui.registerCommand({id:'fetch',title:'读取 GitHub API 状态',handler:async()=>{
    const response=await api.fetch('https://api.github.com/');
    const settings=await api.settings.get('preferences');
    if(settings.show_status)api.log.info(`HTTP ${response.status}`);
  }});
}
export function onunload() {}
