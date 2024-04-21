pub fn wavedash_template(cwd: &str, tab_name: &str, include_wavedash_plugin: bool) -> String {
    let wavedash_plugin = if include_wavedash_plugin {
        r#"
			 pane name="wavedash" {
			 	plugin location="wavedash"
			 }
    	"#
    } else {
        ""
    };

    let template = format!(
        r#"
layout {{
	tab cwd="{cwd}" name="{tab_name}" hide_floating_panes=true {{
	 	pane split_direction="vertical" {{
			pane size="65%" focus=true name="editor" {{
				command "hx"
				args "."
			}}
			pane stacked=true {{
				pane name="cheatsheet" {{
					command "glow"
					args "/home/spc/.config/helix/cheatsheet.md"
				}}
				pane name="tasks" {{
				 	command "task"
				 	args "ls" "limit:20"
				}}
				pane name="tests" {{
				 	command "bacon"
				 	args "test" "-s"
				}}
				pane name="clippy" {{
				 	command "bacon"
				 	args "clippy" "-s"
				}}
				pane name="log" {{
					command "tail"
					args "/tmp/zellij-1000/zellij-log/zellij.log" "-F"
				}}
			}}
		}}
		pane size=1 borderless=true {{
	        plugin location="tab-bar"
	    }}
		floating_panes {{
			{wavedash_plugin}
		}}
	}}
}}
"#,
    );

    template
}
