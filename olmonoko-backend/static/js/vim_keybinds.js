/** @type {"normal"|"insert"|"goto"|"search"} */
var oono_vk_mode = "normal";
var oono_vk_all_modes = ["normal", "insert", "goto", "search"];
var oono_vk_max_mode_length = Math.max(...oono_vk_all_modes.map(m => m.length));

var oono_vk_buffer = "";
var oono_vk_buffer_history = [];

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
		if (e.key == "h") {
			vk_emit_control_signal("move_left");
		}
		else if (e.key == "j") {
			vk_emit_control_signal("move_down");
		}
		else if (e.key == "k") {
			vk_emit_control_signal("move_up");
		}
		else if (e.key == "l") {
			vk_emit_control_signal("move_right");
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
		if (e.detail.data == "now") {
			// noop
		}
		else {
			let data = e.detail.data;
			url = `/?goto=${data}`;
		}

		let link = document.createElement("a");
		let content = document.getElementById("content");
		content.appendChild(link);
		link.href = url;
		// allow htmx to insert boosted behaviour
		htmx.process(link);
		// wait for htmx to finish
		window.requestAnimationFrame(() => {
			link.click();
		});
	}
}

document.addEventListener("olmonoko_vk", vk_handle_event);
