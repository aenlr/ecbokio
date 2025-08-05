# EasyCashier - Bokio

Ett program för att importera Z-Rapporter (dagsrapport/dagskassa) från
[EasyCashier](https://easycashier.se) till [Bokio](https://bokio.se).

## Funktioner

* Kontrollerar redan bokförda Z-Rapporter vilket också upptäcker när SIE-filer
  från EasyCashier redan importeras i Bokio.
* Laddar upp Z-Rapporten i PDF-format som underlag till verifikatet.
* Underlag (PDF och JSON) sparas som filer lokalt.
* Meny för att välja vad som ska importeras.

## Användning

Programmet kan köras utan argument och kommer att be om den information som krävs.

Ska man sedan köra det regelbundet kan man enklast använda miljövariabler för att inte
behöva ange all information varje gång.

```text
ecbokio [OPTIONS]

Options:
  --easycashier-username NAME  Användarnamn för EasyCashier. (EASYCASHIER_USERNAME)
  --easycashier-password NAME  Lösenord för EasyCashier. (EASYCASHIER_PASSWORD)
  --orgnummer ORGNR            Företagets organisationsnummer i EasyCashier (EASYCASHIER_COMPANY)
                               Om detta inte anges så används förvalt företag i EasyCashier.
  --bokio-api-token TOKEN      Token för privat integration i Bokio (BOKIO_API_TOKEN).
  --bokio-company-id UUID      Företagets ID i Bokio (BOKIO_COMPANY_ID).
                               OBS: Detta är inte företagets organisationsnummer utan det ID
                               som står i URL:en när man är inloggad i Bokio.

  --start YYYY-MM-DD           Startdatum för Z-Rapporter (standard är dagens datum)
  --end YYYY-MM-DD             Slutdatum för Z-Rapporter (standard är samma som startdatum)
  --date YYYY-MM-DD            Bearbeta Z-Rapporter för ett specifikt datum (standard är dagens datum)
```

## Guide

### EasyCashier

Gå till [Konto -> Användare](https://backoffice.easycashier.se/control-panel/users)
och lägg till en ny användare. Ange t.ex. bokio som användarnamn.

Ange roll `Företagsadministratör`. Tyvärr måste denna roll användas, det finns ingen
annan roll med tillräcklig behörighet för att läsa Z-Rapporter.

Om du bara har ett företag med bokföring i Bokio så kan du ange `Förvalt företag`
så behöver det inte anges när programmet körs. Kryssa också i `Begränsa rättighet
till förvalt företag`.

Spara och verifiera användaren för det nya kontot.

Spara användare/lösen på säker plats. Ange dessa när programmet körs.

**Varför inte användda sin vanliga användare?**  
Att använda sin vanliga användare är inte att rekommendera så att man inte behöver
använda sitt vanliga lösenord enkelt kan stänga av användaren som används för import
eller byta lösenord utan att den vanliga användaren påverkas.

### Bokio

Skapa en privat integration under `Inställningar -> API Tokens`.
Ange t.ex. EasyCashier som integrationsnamn och klicks på *Lägg till integration*.

Kopiera token och spara på säker plats.

Kontrollera adressen i webbläsaren, den ser ut som något i stil med
`https://app.bokio.se/COMPANY-ID/settings-r/overview`.
Kopiera delen `COMPANY-ID` från adressen i webbläsaren som är delen
efter `https://app.bokio.se/` och nästföljande `/` (snedstreck).
Detta är ditt företags ID i Bokio (company id). Du behöver detta
när programmet körs.

### Miljövaribler

De användarnamn, lösenord, tokens och identiteter som du samlat ihop i 
från EasyCashier och Bokio kan du spara i en fil för att slippa ange dem
varje gång. Se till att bara behöriga har tillgång till filen.

#### Exempel på Windows

Skapa filen `zrappimp.cmd` med följande innehåll och spara filen på samma ställe
som ecbokio.exe

```bat
set EASYCASHIER_USERNAME=xxx
set EASYCASHIER_PASSWORD=xxx
set EASYCASHIER_COMPANY=orgnr
set BOKIO_API_TOKEN=xxx
ecbokio
```

Ersätt `xxx` i filen med respektive värde för ditt företag.

Dubbelklicka på filen för att köra importer.

#### Exempel på Linux, macOs eller annat Un*x-likt OS

Skapa filen `zrappimp.sh` med följande innehåll:

```shell
export EASYCASHIER_USERNAME=xxx
export EASYCASHIER_PASSWORD=xxx
export EASYCASHIER_COMPANY=orgnr
export BOKIO_API_TOKEN=xxx
ecbokio
```

Ersätt `xxx` i filen med respektive värde för ditt företag.

Gör filen exekverbar och endast åtkomlig för dig själv: `chmod 0700 zrappimp.sh`.

Kör scriptet för att importera: `./zrappimp.sh`
