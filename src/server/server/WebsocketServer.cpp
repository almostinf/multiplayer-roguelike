#include "WebsocketServer.h"

#include <algorithm>
#include <functional>
#include <iostream>


//The name of the special JSON field that holds the message type for messages
#define MESSAGE_FIELD "__MESSAGE__"
#define CONNECTION_FIELD "__CONN__"
#define MAP_FIELD "__MAP__"
#define IS_MAP_FIELD "__IS_MAP__"
#define IS_NAME "__IS_NAME__"
#define RATING "__RATING__"
#define TRACK_ME "__TRACK_ME__"
#define CHANGE "__CHANGE__"


Json::Value WebsocketServer::parseJson(const string& json)
{
	Json::Value root;
	Json::Reader reader;
	reader.parse(json, root);
	return root;
}

string WebsocketServer::stringifyJson(const Json::Value& val)
{
	//When we transmit JSON data, we omit all whitespace
	Json::StreamWriterBuilder wbuilder;
	wbuilder["commentStyle"] = "None";
	wbuilder["indentation"] = "";
	
	return Json::writeString(wbuilder, val);
}

WebsocketServer::WebsocketServer()
{
	//Wire up our event handlers
	this->endpoint.set_open_handler(std::bind(&WebsocketServer::onOpen, this, std::placeholders::_1));
	this->endpoint.set_close_handler(std::bind(&WebsocketServer::onClose, this, std::placeholders::_1));
	this->endpoint.set_message_handler(std::bind(&WebsocketServer::onMessage, this, std::placeholders::_1, std::placeholders::_2));
	
	//Initialise the Asio library, using our own event loop object
	this->endpoint.init_asio(&(this->eventLoop));
}

void WebsocketServer::run(int port)
{
	//Listen on the specified port number and start accepting connections
	this->endpoint.listen(port);
	this->endpoint.start_accept();
	
	//Start the Asio event loop
	this->endpoint.run();
}

size_t WebsocketServer::numConnections()
{
	//Prevent concurrent access to the list of open connections from multiple threads
	std::lock_guard<std::mutex> lock(this->connectionListMutex);
	
	return this->openConnections.size();
}

void WebsocketServer::sendMessage(ClientConnection conn, const string& messageType, const string& key)
{
	//Copy the argument values, and bundle the message type into the object
	Json::Value messageData;
	messageData[key] = messageType;
	
	//Send the JSON data to the client (will happen on the networking thread's event loop)
	this->endpoint.send(conn, WebsocketServer::stringifyJson(messageData), websocketpp::frame::opcode::text);
}

void WebsocketServer::broadcastMessage(const string& messageType, const string& key)
{
	//Prevent concurrent access to the list of open connections from multiple threads
	std::lock_guard<std::mutex> lock(this->connectionListMutex);
	
	for (auto conn : this->openConnections) {
		this->sendMessage(conn, messageType, key);
	}
}

void WebsocketServer::onOpen(ClientConnection conn)
{
	{
		//Prevent concurrent access to the list of open connections from multiple threads
		std::lock_guard<std::mutex> lock(this->connectionListMutex);
		
		//Add the connection handle to our list of open connections
		this->openConnections.push_back(conn);
	}

	//Invoke any registered handlers
	for (auto handler : this->connectHandlers) {
		handler(conn);
	}
}

void WebsocketServer::onClose(ClientConnection conn)
{
	{
		//Prevent concurrent access to the list of open connections from multiple threads
		std::lock_guard<std::mutex> lock(this->connectionListMutex);
		
		//Remove the connection handle from our list of open connections
		auto connVal = conn.lock();
		auto newEnd = std::remove_if(this->openConnections.begin(), this->openConnections.end(), [&connVal](ClientConnection elem)
		{
			//If the pointer has expired, remove it from the vector
			if (elem.expired() == true) {
				return true;
			}
			
			//If the pointer is still valid, compare it to the handle for the closed connection
			auto elemVal = elem.lock();
			if (elemVal.get() == connVal.get()) {
				return true;
			}
			
			return false;
		});
		
		//Truncate the connections vector to erase the removed elements
		this->openConnections.resize(std::distance(openConnections.begin(), newEnd));

		if (this->openConnections.size() == 0) {
			this->maps.clear();
			this->names.clear();
		}
	}

	//Invoke any registered handlers
	for (auto handler : this->disconnectHandlers) {
		handler(conn);
	}
}

void WebsocketServer::onMessage(ClientConnection conn, WebsocketEndpoint::message_ptr msg)
{
	//Validate that the incoming message contains valid JSON
	Json::Value messageObject = WebsocketServer::parseJson(msg->get_payload());
	if (messageObject.isNull() == false)
	{
		//Validate that the JSON object contains the message type field
		if (messageObject.isMember(MESSAGE_FIELD))
		{
			//Extract the message type and remove it from the payload
			std::string messageType = messageObject[MESSAGE_FIELD].asString();
			messageObject.removeMember(MESSAGE_FIELD);
			std::string key = MESSAGE_FIELD;
			this->broadcastMessage(messageType, key);
		}
		else if (messageObject.isMember(CONNECTION_FIELD))
		{
			std::string num_of_connection = std::to_string(this->openConnections.size());
			std::string key = CONNECTION_FIELD;
			this->sendMessage(conn, num_of_connection, key);
		}
		else if (messageObject.isMember(IS_MAP_FIELD)) 
		{
			std::string messageType = messageObject[IS_MAP_FIELD].asString();

			size_t num_of_map = std::stoi(messageType);

			std::string response;
			if (num_of_map <= this->maps.size())
				response = maps[num_of_map - 1];
			else 
				response = "F";

			// std::cout << num_of_map << '\n';
			
			std::string key = IS_MAP_FIELD;
			this->sendMessage(conn, response, key);
		}
		else if (messageObject.isMember(MAP_FIELD))
		{
			std::string source = msg->get_payload();
			std::string messageType = source.substr(12, source.size() - 13 - 1);
			maps.push_back(messageType);
		}
		else if (messageObject.isMember(IS_NAME)) 
		{
			std::string messageType = messageObject[IS_NAME].asString();
			std::string response;

			if (std::find(names.begin(), names.end(), messageType) != names.end()) {
				response = "F";
			} 
			else {
				response = "T";
				names.push_back(messageType);
			}
			std::string key = IS_NAME;
			this->sendMessage(conn, response, key);
		}
		else if (messageObject.isMember(RATING)) {
			std::string response;
			for (auto const& player : rating) {
				response.append(player.first);
				response.push_back(':');
				response.append(player.second);
				response.push_back(' ');
			}
			std::string key = RATING;
			this->sendMessage(conn, response, key);
		}
		else if (messageObject.isMember(TRACK_ME)) {
			std::string messageType = messageObject[TRACK_ME].asString();
			std::string name;
			std::string num;
			bool check = false;
			for (auto ch : messageType) {
				if (ch == ' ') {
					check = true;
				}
				else if (!check) {
					name.push_back(ch);
				} 
				else {
					num.push_back(ch);
				}
			}

			if (rating.find(name) == rating.end()) {
				rating.insert(std::make_pair(name, num));
			} 
			else {
				rating[name] = num;
			}
		}
		else if (messageObject.isMember(CHANGE)) {
			std::string messageType = messageObject[CHANGE].asString();
			messageObject.removeMember(CHANGE);
			std::string key = CHANGE;
			this->broadcastMessage(messageType, key);
		}
	}
}
