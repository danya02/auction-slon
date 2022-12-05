# Todo: change makefile for two environments
basedir = "."

all: build run

build: front back

front:
	cd $(basedir)/frontend; trunk build

back:
	cd $(basedir)/backend; cargo build

run: front
	cd $(basedir)/backend; cargo run
