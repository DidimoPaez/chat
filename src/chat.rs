use std::collections::HashMap;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
#[derive(Debug)]
enum Errores {
    Error(String),
}
pub struct Chat {
    usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
}

impl Chat {
    pub fn new() -> Result<Self, Errores> {
        Ok(Chat {
            usuarios: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn ejecutar(&mut self, listener: TcpListener) -> Result<(), Errores> {
        while let Ok((stream, _)) = listener.accept() {
            let usuarios_clone = Arc::clone(&self.usuarios);
            let stream = Arc::new(Mutex::new(stream));

            thread::spawn(move || {
                // Manejar conexión con el cliente
                if let Err(e) = handle_connection(usuarios_clone, stream) {
                    eprintln!("Error en la conexión: {:?}", e);
                }
            });
        }
        Ok(())
    }
}
/* Se ejecuta con: cargo run --bin chat
*/
fn main() -> Result<(), Errores> {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .map_err(|e| Errores::Error(format!("Falla creando el socket: {}", e)))?;
    println!("Servidor funcionando. Esperando conexiones..");
    let mut chat = Chat::new()?;
    chat.ejecutar(listener)?;

    Ok(())
}

fn leer_stream(stream: Arc<Mutex<TcpStream>>) -> Result<(String, usize), Errores> {
    let mut buffer_lectura = [0; 1024];
    let mut stream_locked = stream.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock: {}", e)))?;
    let n = stream_locked.read(&mut buffer_lectura).map_err(|e| Errores::Error(format!("No se pudo leer del stream_locked: {}", e)))?;
    println!("ESTO ES LEIDO DESDE LERR_STREAM : {}", String::from_utf8_lossy(&buffer_lectura[..n]).trim().to_string()); /// ///////////////////////PRINT PRUEBA
    Ok((String::from_utf8_lossy(&buffer_lectura[..n]).trim().to_string(), n))
}
fn leer_stream_usuario(usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>, stream: Arc<Mutex<TcpStream>>, nickname: String) -> Result<Option<String>, Errores> {
    println!("LLEGO ALGO PARA SER LEIDO DEL STREAM DE: {}", nickname.to_string()); /// ///////////////////////PRINT PRUEBA
    let (msg, n) = leer_stream(Arc::clone(&stream))?;
    println!("EL USUARIO: {}, ENVIO UN MESAJE LEIDO: {}", nickname.to_string(), msg.to_string()); /// ///////////////////////PRINT PRUEBA
    if n == 0 {
        println!("Cliente {} se ha desconectado.", nickname);
        if let Ok(mut usuarios_lock) = usuarios.lock() {
            usuarios_lock.remove(&nickname);
        }
        return Ok(None);
    }

    if msg.trim().is_empty() {
        println!("El usuario {} envió un mensaje vacío o con solo espacios, ignorando...", nickname);
        return Ok(Some("".to_string())); // Continúa el ciclo sin cortar la conexión
    }
    Ok(Some(msg))
}
fn escribir_stream(stream: Arc<Mutex<TcpStream>>, msg: &[u8]) -> Result<(), Errores> {
    let mut stream_locked = stream.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock: {}", e)))?;
    stream_locked.write_all(msg).map_err(|e| Errores::Error(format!("No se pudo escribir el stream_locked: {}", e)))?;
    stream_locked.flush().map_err(|e| Errores::Error(format!("No se pudo hacer flush del stream_locked: {}", e)))?;
    Ok(())
}
fn verificar_usuario(usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>, nickname: String) -> Result<bool, Errores> {
    let mut usuarios_lock = usuarios.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock de usuarios: {}", e)))?;
    Ok(usuarios_lock.contains_key(&nickname))
}
fn handle_connection(usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>, stream: Arc<Mutex<TcpStream>>) -> Result<(), Errores> {
    let (mut nickname, _) = leer_stream(Arc::clone(&stream))?;

    while verificar_usuario(Arc::clone(&usuarios), nickname.to_string())? {
        let msg = "El usuario ya existe, ingrese otro nickname".as_bytes();
        escribir_stream(Arc::clone(&stream), msg)?;
        let (nuevo_nickname, _) = leer_stream(Arc::clone(&stream))?;
        nickname = nuevo_nickname;
    }
    {
        let mut usuarios_lock = usuarios.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock de usuarios: {}", e)))?;
        usuarios_lock.insert(nickname.to_string(), Arc::clone(&stream));
        println!("Nuevo usuario ingresado: {:?}", nickname.to_string());
    }
    loop {
        println!("SE EMPIEZA EL LOOP DEL usuario: {:?}", nickname.to_string()); /// ////////////////////////////////////////////PRINT PRUEBA
        let msg = match leer_stream_usuario(Arc::clone(&usuarios), Arc::clone(&stream), nickname.to_string())? {
            Some(mensaje) if !mensaje.is_empty() => mensaje,
            Some(_) => continue, // Si el mensaje está vacío, simplemente continúa el loop
            None => return Ok(()),
        };
        println!("EL USUARIO: {}, ENVIO UN MESAJE PRIVADO: {}", nickname.to_string(), msg.to_string()); /// ///////////////////////PRINT PRUEBA
        if es_msg_privado(msg.to_string())? {
            let destinatario_msg: Vec<String> = msg.split("**").map(|x| x.to_string()).collect();
            if destinatario_msg.len() != 2 {
                let msg_falla_sintaxis = "El mensaje privado no tiene el formato adecuado".as_bytes();
                escribir_stream(Arc::clone(&stream), msg_falla_sintaxis)?;
            } else {
                println!("EL USUARIO: {}, ENVIO UN MESAJE PRIVADO: {}, AL DESTINATARIO: {}", nickname.to_string(), destinatario_msg[1].to_string(), destinatario_msg[0].to_string()); /// ///////////////////////PRINT PRUEBA
                enviar_msg_privado(Arc::clone(&usuarios), Arc::clone(&stream), destinatario_msg[0].to_string(), destinatario_msg[1].to_string())?;
            }
        } else {
            let usuarios_clone = Arc::clone(&usuarios);
            println!("EL USUARIO {}, ENVIO UN MESAJE AL CHAT {}", nickname.to_string(), msg.to_string()); /// ///////////////////////PRINT PRUEBA
            broadcast(usuarios_clone, msg, nickname.to_string())?;
        }
    }
}

fn es_msg_privado(msg_actual: String) -> Result<bool, Errores> {
    if msg_actual.contains("**") {
        return Ok(true);
    }
    Ok(false)
}

fn enviar_msg_privado(usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>, stream: Arc<Mutex<TcpStream>>, destinatario: String, msg: String) -> Result<(), Errores> {
    let mut usuarios_lock = usuarios.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock de usuarios: {}", e)))?;
    if let Some(destinatario_stream) = usuarios_lock.get_mut(&destinatario) {
        escribir_stream(Arc::clone(destinatario_stream), msg.as_bytes())?;
    } else {
        return Err(Errores::Error("No se pudo enviar msg privado".to_string()));
    }
    Ok(())
}

fn broadcast(usuarios: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>, msg: String, remitente: String) -> Result<(), Errores> {
    let usuarios_lock = usuarios.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock de usuarios: {}", e)))?;

    // Recolectar las conexiones a las que se les enviará el mensaje
    let destinatarios: Vec<Arc<Mutex<TcpStream>>> = usuarios_lock
        .iter()
        .filter(|&(nickname, _)| *nickname != remitente)
        .map(|(_, stream)| Arc::clone(stream))
        .collect();

    drop(usuarios_lock);

    let msg = format!("{}: {}", remitente, msg);
    for stream in destinatarios {
        escribir_stream(stream, msg.as_bytes())?;
    }
    Ok(())
}


