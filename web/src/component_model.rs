use std::string::ToString;
use yew::prelude::*;
use yew::services::reader::{File, FileData, ReaderService, ReaderTask};

use flash_lso::flex;
use flash_lso::types::{Attribute, Sol, Value};
use flash_lso::LSODeserializer;

use crate::blob_bindgen::Blob;
use crate::component_hexview::HexView;
use crate::component_number_input::NumberInput;
use crate::component_tab::Tab;
use crate::component_tabs::Tabs;
use crate::component_treenode::TreeNode;
use crate::uintarray_bindgen::Uint8Array;
use crate::url_bindgen::URL;
use crate::web_expect::WebSafeExpect;
use crate::EditableValue;
use flash_lso::encoder::write_to_bytes;
use std::ops::Deref;
use crate::component_string_input::StringInput;

pub struct LoadedFile {
    pub file_name: String,
    pub file: Option<Sol>,
}

impl LoadedFile {
    pub fn empty_from_file(file: &File) -> Self {
        LoadedFile {
            file: None,
            file_name: file.name().to_string(),
        }
    }
}

pub struct Model {
    link: ComponentLink<Self>,
    reader: ReaderService,
    tasks: Vec<ReaderTask>,
    files: Vec<LoadedFile>,
    current_selection: Option<EditableValue>,
    current_tab: Option<usize>,
}

#[derive(Debug)]
pub enum Msg {
    Files(Vec<File>),
    Loaded(usize, FileData),
    Selection(EditableValue),
    Edited(Value),
    TabSelected(usize),
    CloseTab(usize),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            reader: ReaderService::new(),
            tasks: vec![],
            files: vec![],
            current_selection: None,
            current_tab: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        log::info!("MODEL msg={:?}", msg);
        match msg {
            Msg::Files(files) => {
                for file in files.into_iter() {
                    let index = self.files.len();
                    self.files.push(LoadedFile::empty_from_file(&file));
                    let task = {
                        let callback = self
                            .link
                            .callback(move |file_data| Msg::Loaded(index, file_data));
                        self.reader
                            .read_file(file, callback)
                            .web_expect("Unable to read file")
                    };
                    self.tasks.push(task);
                }
            }
            Msg::Loaded(index, file) => {
                let mut parser = LSODeserializer::default();
                flex::decode::register_decoders(&mut parser.amf3_decoder);

                let sol = parser
                    .parse(&file.content)
                    .web_expect("Failed to parse file")
                    .1;
                self.files
                    .get_mut(index)
                    .web_expect(&format!("No loading file at index {}", index))
                    .file = Some(sol);

                if self.current_tab.is_none() {
                    self.current_tab = Some(0);
                }
            }
            Msg::Selection(val) => self.current_selection = Some(val),
            Msg::Edited(val) => {
                self.current_selection
                    .as_ref()
                    .web_expect("Unable to get current selection")
                    .callback
                    .emit(val.clone());
                self.current_selection
                    .as_mut()
                    .web_expect("Unable to get mut current selection")
                    .value = val;
            }
            Msg::TabSelected(index) => self.current_tab = Some(index),
            Msg::CloseTab(index) => {
                log::warn!("removing file");
                if let Some(sel) = self.current_tab {
                    if sel > 0 {
                        if sel >= index {
                            self.current_tab = Some(sel - 1);
                        }
                    } else {
                        self.current_tab = None;
                    }
                }
                self.files.remove(index);
                log::warn!("File removed");
            }
        }
        true
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                { self.navbar() }
                <Tabs selected={self.current_tab} ontabselect=self.link.callback(move |index| Msg::TabSelected(index)) ontabremove=self.link.callback(move |index| Msg::CloseTab(index))>
                    { for self.files.iter().enumerate().map(|(i,f)| html_nested! {
                    <Tab label={&f.file_name} loading=f.file.is_none()>
                        { if let Some(file) = &f.file {
                            self.view_file(i, file)
                        } else {
                            html! {}
                        }}
                    </Tab>
                }) }
                </Tabs>
            </div>
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        // jquery_bindgen::jquery("#tree").jstree();
    }
}

impl Model {
    fn value_details(&self, val: EditableValue) -> Html {
        match val.value {
            Value::Object(children, Some(def)) => {
                let def_clone = def.clone();
                let dynamic_icon = if def.attributes.contains(Attribute::DYNAMIC) {
                    "icon/check.svg"
                } else {
                    "icon/x.svg"
                };
                let external_icon = if def.attributes.contains(Attribute::EXTERNAL) {
                    "icon/check.svg"
                } else {
                    "icon/x.svg"
                };

                let static_props_details = if def.static_properties.is_empty() {
                    html! {}
                } else {
                    html! {
                    <table class="table table-striped">
                            <thead>
                                <tr>
                                    <th>{"Static Properties"}</th>
                                </tr>
                            </thead>
                            { for def_clone.static_properties.iter().map(|p| html! {
                                <tr>
                                    <td>{p}</td>
                                </tr>
                            })}
                        </table>
                    }
                };

                return html! {
                    <>
                      <div class="input-group mb-2">
                        <div class="input-group-prepend">
                          <div class="input-group-text">{"Name"}</div>
                        </div>
                        <input onchange={ self.link.callback(move |cd| {
                            if let ChangeData::Value(s) = cd {
                                let mut new_def = def.clone();
                                new_def.name = s;
                                Msg::Edited(Value::Object(children.clone(), Some(new_def)))
                            } else {
                                Msg::Edited(Value::Object(children.clone(), Some(def.clone())))
                            }
                        })} value={def.name.clone()} class="form-control" type="text"/>
                      </div>

                      <ul class="list-group list-group-horizontal mt-2 mb-2">
                          <li class="list-group-item"><img src={dynamic_icon} style={"width: 32; height: 32;"} class={"mr-2"}/>{"Dynamic"}</li>
                          <li class="list-group-item"><img src={external_icon} style={"width: 32; height: 32;"} class={"mr-2"}/>{"External"}</li>
                      </ul>
                        { static_props_details }
                    </>
                };
            }
            Value::VectorObject(_, name, _) => html! {
                <>
                <p>{"name"}</p>
                <p>{name}</p>
                </>
            },
            Value::Number(n) => html! {
                <NumberInput onchange=self.link.callback(move |data| Msg::Edited(Value::Number(data))) value={n}/>
            },
            Value::Integer(n) => html! {
                <input onchange={ self.link.callback(move |cd| {
                    if let ChangeData::Value(s) = cd {
                        if let Ok(data) = s.parse::<i32>() {
                            Msg::Edited(Value::Integer(data))
                        } else {
                            Msg::Edited(Value::Integer(n))
                        }
                    } else {
                        Msg::Edited(Value::Integer(n))
                    }
                })} value={n} class="form-control"/>
            },
            Value::ByteArray(n) => html! {
                <HexView bytes={n}/>
            },
            Value::String(s) => html! {
                <input onchange={ self.link.callback(move |cd| {
                    if let ChangeData::Value(s) = cd {
                        Msg::Edited(Value::String(s.clone()))
                    } else {
                        Msg::Edited(Value::String(s.clone()))
                    }
                })} value={s.clone()} class="form-control"/>
            },
            Value::Bool(b) => html! {
                <div class="custom-control custom-switch">
                  <input type={"checkbox"} class={"custom-control-input"} id={"customSwitch1"} checked={b} onclick={self.link.callback(move |_| {
                    Msg::Edited(Value::Bool(!b))
                  })}/>
                  <label class={"custom-control-label"} for={"customSwitch1"}>{"State"}</label>
                </div>
            },
            Value::Date(x, tz) => html! {
                <>
                <div class="input-group mb-2">
                    <div class="input-group-prepend">
                      <div class="input-group-text">{"Epoch"}</div>
                    </div>
                    <input onchange={ self.link.callback(move |cd| {
                        if let ChangeData::Value(s) = cd {
                            if let Ok(x) = s.parse::<f64>() {
                                Msg::Edited(Value::Date(x, tz))
                            } else {
                                Msg::Edited(Value::Date(x, tz))
                            }
                        } else {
                            Msg::Edited(Value::Date(x, tz))
                        }
                    })} value={x} class="form-control" type="number"/>
                  </div>

                  { if tz.is_some() { html!{
                  <div class="input-group mb-2">
                    <div class="input-group-prepend">
                      <div class="input-group-text">{"Timezone"}</div>
                    </div>
                    <input onchange={ self.link.callback(move |cd| {
                        if let ChangeData::Value(s) = cd {
                            if let Ok(tz) = s.parse::<u16>() {
                                Msg::Edited(Value::Date(x, Some(tz)))
                            } else {
                                Msg::Edited(Value::Date(x, tz))
                            }
                        } else {
                            Msg::Edited(Value::Date(x, tz))
                        }
                    })} value={tz.web_expect("Unable to get timezone")} class="form-control" type="number"/>
                  </div>
                  }} else {html!{}}}
                </>
            },
            Value::XML(content, string) => html! {
                <StringInput onchange=self.link.callback(move |s| Msg::Edited(Value::XML(s, string))) value={content.clone()}/>
            },
            Value::VectorInt(elements, fixed_length) => {
                let elements_clone = elements.clone();
                let elements_clone2 = elements.clone();
                let elements_clone3 = elements.clone();
                return html! {
                    <>
                        <div class="custom-control custom-switch mb-2">
                          <input type={"checkbox"} class={"custom-control-input"} id={"vectorIntFixed"} checked={fixed_length} onclick={self.link.callback(move |_| {
                            Msg::Edited(Value::VectorInt(elements_clone.clone(), !fixed_length))
                          })}/>
                          <label class={"custom-control-label"} for={"vectorIntFixed"}>{"Fixed Length"}</label>
                        </div>

                        <table class="table table-striped">
                            <thead>
                                <tr>
                                    <th>{"#"}</th>
                                    <th>{"Value"}</th>
                                    <th></th>
                                    <th></th>
                                </tr>
                            </thead>
                            <tbody>
                            { for elements.iter().enumerate().map(|(i, e)| {
                                let elements_clone4 = elements_clone3.clone();
                                let elements_clone5 = elements_clone3.clone();
                                html! {
                                <tr>
                                    <td>{i}</td>
                                    <td>
                                        <input onchange={ self.link.callback(move |cd| {
                                            if let ChangeData::Value(s) = cd {
                                                if let Ok(data) = s.parse::<i32>() {
                                                    let mut new_elements = elements_clone5.clone();
                                                    new_elements[i] = data;
                                                    Msg::Edited(Value::VectorInt(new_elements, fixed_length))
                                                } else {
                                                    Msg::Edited(Value::VectorInt(elements_clone5.clone(), fixed_length))
                                                }
                                            } else {
                                                Msg::Edited(Value::VectorInt(elements_clone5.clone(), fixed_length))
                                            }
                                        })} value={e} class="form-control" type="text"/>
                                    </td>
                                    <td></td>
                                    <td>
                                    <span onclick={self.link.callback(move |_| {
                                        let mut e = elements_clone4.clone();
                                        e.remove(i);
                                        Msg::Edited(Value::VectorInt(e, fixed_length))
                                      })} class="btn btn-link">
                                            <img src={"icon/x.svg"} style={"width: 32; height: 32;"} class={"mr-2"}/>
                                        </span>
                                    </td>
                                </tr>
                            }
                            })}
                            </tbody>
                        </table>
                        <span onclick={self.link.callback(move |_| {
                            let mut e = elements_clone3.clone();
                            e.push(0);
                            log::info!("Updating vectorint");
                            Msg::Edited(Value::VectorInt(e, fixed_length))
                          })} class="btn btn-primary">{"Add"}</span>
                    </>
                };
            }
            // Value::AMF3(e) => self.value_details(e.clone()),
            _ => html! {},
        }
    }

    fn navbar(&self) -> Html {
        html! {
            <nav class="navbar navbar-expand-lg">
                <ul class="navbar-nav mr-auto">
                    <li class="nav-item">
                        <div class="btn-group mr-2" role="group">
                            <label for="files" class="btn btn-primary">{"Open"}</label>
                            { self.save_button() }
                        </div>
                    </li>
                    <input id="files" style="visibility:hidden;" type="file" onchange=self.link.callback(move |value| {
                                    let mut result = Vec::new();
                                    if let ChangeData::Files(files) = value {
                                        let files = js_sys::try_iter(&files)
                                            .web_expect("Unable to try_iter files")
                                            .web_expect("Unable to try_iter files 2")
                                            .into_iter()
                                            .map(|v| File::from(v.web_expect("File from")));
                                        result.extend(files);
                                    }
                                    Msg::Files(result)
                                })/>
                </ul>
            </nav>
        }
    }

    fn save_button(&self) -> Html {
        if let Some(tab_index) = self.current_tab {
            let bytes = write_to_bytes(self.files[tab_index].file.as_ref().unwrap());

            let options: js_sys::Object = js_sys::Object::new();

            let arr: Uint8Array = Uint8Array::new(bytes.len() as u32);
            for (i, b) in bytes.iter().enumerate() {
                arr.set(i as u32, (*b).into());
            }

            let arr2: js_sys::Array = js_sys::Array::new_with_length(1);
            arr2.set(0, arr.into());

            let blob = Blob::new(arr2, options.into());
            let url = URL::createObjectURL(&blob);

            return html! {
                <a href={url} download={"save.sol"} class="btn btn-primary" style="height: 38px">{"Save"}</a>
            };
        } else {
            return html! {};
        }
    }

    fn view_file(&self, _index: usize, data: &Sol) -> Html {
        html! {
            <div class="container-fluid">
                <div class="row">
                    <div class="col-5">
                        <ul class="list-group list-group-horizontal mt-2">
                          <li class="list-group-item"><img src={"icon/database.svg"} style={"width: 32; height: 32;"} class={"mr-2"}/>{data.header.length}</li>
                          <li class="list-group-item">{data.header.format_version}</li>
                        </ul>

                        <div id="tree">
                            <span><img src={"icon/file.svg"} style={"width: 32; height: 32;"} class={"mr-2"}/>{"/"}</span>
                            <ul>
                                { for data.body.iter().map(|e| html! {
                                    <TreeNode name={e.name.clone()} value={e.value.deref().clone()} parent_callback={self.link.callback(|val| Msg::Selection(val))}></TreeNode>
                                })}
                            </ul>
                        </div>
                    </div>
                    <div class="col-7">
                        {
                            if let Some(selection) = &self.current_selection {
                                let details_content = self.value_details(selection.clone());
                                let value_type = match &selection.value {
                                    Value::Number(_) => "Number".to_string(),
                                    Value::Bool(_) => "Boolean".to_string(),
                                    Value::String(_) => "String".to_string(),
                                    Value::Object(_, _) => "Object".to_string(),
                                    Value::Null => "Null".to_string(),
                                    Value::Undefined => "Undefined".to_string(),
                                    Value::ECMAArray(_, _, _) => "ECMAArray".to_string(),
                                    Value::StrictArray(_) => "StrictArray".to_string(),
                                    Value::Date(_, _) => "Date".to_string(),
                                    Value::Unsupported => "Unsupported".to_string(),
                                    Value::XML(_, _) => "XML".to_string(),
                                    Value::AMF3(_) => "AMF3<TODO>".to_string(),
                                    Value::Integer(_) => "Integer".to_string(),
                                    Value::ByteArray(_) => "ByteArray".to_string(),
                                    Value::VectorInt(_, _) => "Vector<Int>".to_string(),
                                    Value::VectorUInt(_, _) => "Vector<UInt>".to_string(),
                                    Value::VectorDouble(_, _) => "Vector<Double>".to_string(),
                                    Value::VectorObject(_, _, _) => "Vector<Object>".to_string(),
                                    Value::Dictionary(_, _) => "Dictionary".to_string(),
                                    Value::Custom(_, _, cd) => {
                                        if let Some(cd) = cd {
                                            format!("Custom<{}>", cd.name)
                                        } else {
                                            "Custom<Unknown>".to_string()
                                        }
                                    },
                                     _ => "Unknown".to_string()
                                };

                                html! {
                                    <>
                                    <ul class="list-group list-group-horizontal mt-2 mb-2">
                                      <li class="list-group-item">{value_type}</li>
                                    </ul>
                                    {{details_content}}
                                    </>
                                }
                            } else {
                                html! { <p>{"Select an item"}</p> }
                            }
                        }
                    </div>
                </div>
            </div>
        }
    }
}
