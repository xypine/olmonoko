/** @type {"normal"|"insert"|"goto"|"search"} */
var oono_vk_mode = "normal";
var oono_vk_all_modes = ["normal", "insert", "goto", "search"];
var oono_vk_max_mode_length = Math.max(...oono_vk_all_modes.map(m => m.length));

var oono_vk_buffer = "";
var oono_vk_buffer_history = [];

var oono_vk_hint = "";

/**
 * @param {string} signal name of the signal to emit
 * @param {string} [data] data to send with the signal
 */
function vk_emit_control_signal(signal, data = null) {
	var event = new CustomEvent("olmonoko_vk", {
		detail: {
			signal,
			data
		}
	});
	console.log("Emitting control signal:", signal, data);
	document.dispatchEvent(event);
}

/** 
 * @param {"normal"|"insert"|"goto"|"search"} mode the mode to set the editor to
 */
function vk_reset_mode(mode = "normal") {
	if (mode != oono_vk_mode) {
		console.log("Changing mode to", mode);
	}
	oono_vk_mode = mode;
	oono_vk_buffer = "";
	oono_vk_buffer_history = [];
	oono_vk_hint = "";
}


async function vk_update_insert_preview() {
	if(!oono_vk_buffer.length || oono_vk_mode !== "insert") {
		oono_vk_hint = "";
		return;
	}
	const input_used = `${oono_vk_buffer}`;
	if(!oono_vk_hint.endsWith("...")) {
		oono_vk_hint += " >> Loading...";
	}
	try {
		const preview_result = await fetch(
			`${window.location.origin}/api/ui_utils/nlcep?nl=${input_used}`
		);
		let summary = "?";
		let time = "?";
		let location = "?";
		let duration = "?";
		let extra = " ";
		if(preview_result.ok) {
			const json = await preview_result.json();
			console.info({json});
			summary = json.summary;
			time = `${json.date} ${json.time ? json.time : ""}`;
			if(json.location) {
				location = json.location;
			}
			if(json.duration) {
				duration = json.duration;
			}
		} else {
			const text = await preview_result.text();
			summary = input_used;
			extra = `Hint: ${text}`;
		}
		if(oono_vk_mode === "insert") {
			oono_vk_hint = `What: ${summary}\nWhen: ${time}\nWhere: ${location}\nFor how long: ${duration}\n${extra}`;
		}
	} catch(e) {
		console.error(e);
		if(oono_vk_mode === "insert") {
			oono_vk_hint = `Error: ${e}`;
		}
	}
	vk_render();
}

/**
 * @param {KeyboardEvent} e
 */
function vk_modify_buffer(e) {
	if (e.key == "Control" || e.key == "Meta" || e.key == "Alt" || e.key == "Shift") {
		return false; // this keypress was not handled
	}
	if (e.key == "Backspace") {
		if (oono_vk_buffer_history.length > 0) {
			oono_vk_buffer = oono_vk_buffer_history.pop();
		}
	}
	else {
		oono_vk_buffer_history.push(oono_vk_buffer);
		oono_vk_buffer += e.key;
	}
	console.log("Buffer:", oono_vk_buffer);
	vk_update_insert_preview();
	return true; // this keypress was handled
}

/**
 * @param {KeyboardEvent} e
 */
function vk_handle_keypress(e) {
	if (oono_vk_mode == "normal") {
		if (e.ctrlKey || e.metaKey || e.altKey) {
			return false; // this keypress was not handled
		}
		if (document.querySelector("input:focus") || document.querySelector("textarea:focus")) {
			return false; // this keypress was not handled
		}

		const calendar = document.getElementById("calendar");
		if (calendar && e.key == "h") {
			vk_emit_control_signal("move", "left");
		}
		else if (calendar && e.key == "j") {
			vk_emit_control_signal("move", "down");
		}
		else if (calendar && e.key == "k") {
			vk_emit_control_signal("move", "up");
		}
		else if (calendar && e.key == "l") {
			vk_emit_control_signal("move", "right");
		}

		else if (e.key == "i") {
			vk_reset_mode("insert");
		}
		else if (e.key == "g") {
			vk_reset_mode("goto");
		}
		else if (e.key == "/") {
			vk_reset_mode("search");
		}
		else {
			return false; // this keypress was not handled
		}
	}
	else if (oono_vk_mode == "insert") {
		if (e.key == "Escape") {
			vk_reset_mode();
		}
		else if (e.key == "Enter") {
			vk_emit_control_signal("insert", oono_vk_buffer);
			vk_reset_mode();
		}
		else {
			return vk_modify_buffer(e);
		}
	}
	else if (oono_vk_mode == "goto") {
		if (e.key == "Escape") {
			vk_reset_mode();
		}
		else if (e.key == "Enter") {
			vk_emit_control_signal("goto", oono_vk_buffer);
			vk_reset_mode();
		}
		else if (e.key == "g" && oono_vk_buffer.length == 0) {
			vk_emit_control_signal("goto", "now");
			vk_reset_mode();
		}
		else {
			return vk_modify_buffer(e);
		}
	}
	else if (oono_vk_mode == "search") {
		if (e.key == "Escape") {
			vk_reset_mode();
		}
		else if (e.key == "Enter") {
			vk_emit_control_signal("search", oono_vk_buffer);
			vk_reset_mode();
		}
		else {
			return vk_modify_buffer(e);
		}
	}
	else {
		// This should never happen
		console.error("Invalid mode:", oono_vk_mode);
	}

	return true; // this keypress was handled
}

function vk_render() {
	let overlay = document.getElementById("vk_overlay");
	if (oono_vk_mode != "normal") {
		let mode = document.getElementById("vk_mode");
		let buffer = document.getElementById("vk_buffer");
		mode.innerText = oono_vk_mode.padEnd(oono_vk_max_mode_length, " ");

		if (oono_vk_mode == "goto") {
			// add a slash after the year and month
			let as_number = parseInt(oono_vk_buffer);
			// check if valid number
			if (oono_vk_buffer.length > 0 && oono_vk_buffer.length < 9 && !oono_vk_buffer.includes(" ") && as_number < 99991231 && as_number >= -99991231) {
				let out = "";
				let inp = oono_vk_buffer;
				if (inp.startsWith("-")) {
					inp = inp.substring(1);
					out += "-";
				}
				for (let i = 0; i < inp.length; i++) {
					if (i == 4 || i == 6) {
						out += "-";
					}
					out += inp[i];
				}
				buffer.innerText = out;
			}
			else {
				buffer.innerText = oono_vk_buffer;
			}
		} else {
			buffer.innerText = oono_vk_buffer;
		}
		buffer.innerText += "_";

		overlay.classList = ["open"];
	}
	else {
		overlay.classList = [];
	}

	let hintbox = document.getElementById("vk_hint");
	if(hintbox) {
		hintbox.innerText = oono_vk_hint;
		if (oono_vk_mode !== "normal" && oono_vk_hint.length) {
			hintbox.classList = ["open"];
		} else {
			hintbox.classList = [];
		}
	}
}

document.onkeydown = function(e) {
	if (vk_handle_keypress(e)) {
		e.preventDefault();
		e.stopPropagation();
	}
	vk_render();
}

function vk_handle_event(e) {
	if (e.detail.signal == "goto") {
		console.log("Received goto signal:", e.detail.data);
		let url = `/`;
		let spa = true;
		if (e.detail.data == "now") {
			spa = false;
		}
		else {
			let data = e.detail.data;
			url = `/?goto=${data}`;
		}

		let link = document.createElement("a");
		let content = document.getElementById("content");
		content.appendChild(link);
		link.href = url;
		if (spa) {
			// allow htmx to insert boosted behaviour
			htmx.process(link);
		}
		// wait for htmx to finish
		window.requestAnimationFrame(() => {
			link.click();
		});
	}
	else if (e.detail.signal == "move") {
		console.log("Received move signal:", e.detail.data);
		let data = e.detail.data;
		if (data == "left") {
			document.getElementById("week-prev")?.click();
		}
		else if (data == "right") {
			document.getElementById("week-next")?.click();
		}
	}
	else if (e.detail.signal == "insert") {
		console.log("Received insert signal:", e.detail.data);
		window.location.href = `${window.location.origin}/local?nl=${e.detail.data}`;
	}
}

document.addEventListener("olmonoko_vk", vk_handle_event);
