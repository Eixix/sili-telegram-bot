import socket

HOST = "server.com" #server hostname or ip address
PORT = 1234 #port used by the server

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((HOST, PORT))
    s.sendall(b"Hello, World")
    data = s.recv(1024)

print(f"Received {data!r}")
