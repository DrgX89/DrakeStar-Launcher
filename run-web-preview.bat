@echo off
title DrakeStar Launcher - Visual CyberDeck Preview
cd /d "%~dp0\ui"
echo Iniciando DrakeStar UI en modo reactivo...
npm run dev -- --open
