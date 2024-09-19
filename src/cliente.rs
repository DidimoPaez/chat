use std::env::args;
use std::{io, string, thread};
use std::io::{stdin, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
enum Errores {
    Error(String),
}
/*  se ejecuta con: cargo run --bin cliente 127.0.0.1 8080
*/
fn main() -> Result<(), Errores> {
    let addr: Vec<String> = args().collect();
    if addr.len() != 3 {
        println!("Cantidad de argumentos inválido");
        println!("{:?} se debe ingresar <host> <port>", addr[0]);
        return Err(Errores::Error("Falla en cant de args".to_string()));
    }
    let sckt_addr = format!("{}:{}", addr[1].trim(), addr[2].trim());

    let mut stream = TcpStream::connect(sckt_addr)
        .map_err(|e| Errores::Error(format!("Falla creando el socket: {}", e)))?;
    let mut stream = Arc::new(Mutex::new(stream));

    println!("Ingrese su nickname:");
    let mut nickname = String::new();
    std::io::stdin().read_line(&mut nickname).map_err(|e| Errores::Error(format!("Falla leyendo el socket en cliente: {}", e)))?;
    {
        let mut stream_locked = stream.lock().unwrap();
        stream_locked.write_all(nickname.trim().as_bytes()).map_err(|e| Errores::Error(format!("Falla escribiendo nickname en el socket. from cliente: {}", e)))?;
    }
    let stream_clone = Arc::clone(&stream);

    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            let msg = {
                // Bloquear el stream para leer
                let mut stream_locked = match stream_clone.lock() {
                    Ok(lock) => lock,
                    Err(e) => {
                        println!("No se pudo obtener el lock. {:?}", e);
                        break;
                    }
                };

                let n = match stream_locked.read(&mut buffer) {
                    Ok(n) => n,
                    Err(e) => {
                        println!("No se pudo leer del socket: {:?}", e);
                        break;
                    }
                };

                if n == 0 {
                    // Si no hay más datos, el servidor se desconectó o hubo un error
                    println!("El servidor se ha desconectado.");
                    break;
                }
                String::from_utf8_lossy(&buffer[..n])
            };
            // Mostrar el mensaje en la terminal
            println!("{}", msg);
        }
    });

    loop {
        let mut buffer_escritura = String::new();
        io::stdin().read_line(&mut buffer_escritura).map_err(|e| Errores::Error(format!("Falla leyendo el socket en cliente: {}", e)))?;
        println!("SE ESTA INTENTANDO ENVIAR ALGO POR EL LOOP {}\
        de tamaño{:?}", &buffer_escritura, &buffer_escritura.trim().len()); /// ///////////////////////PRUEBA
        let mut stream_lock = stream.lock().map_err(|e| Errores::Error(format!("No se pudo obtener el lock. {:?}", e)))?;
        stream_lock.write_all(buffer_escritura.trim().as_bytes()).map_err(|e| Errores::Error(format!("Falla escribiendo en el socket. from cliente: {}", e)))?;
        drop(stream_lock);
    }
}