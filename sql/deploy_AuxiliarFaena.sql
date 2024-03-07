CREATE DATABASE AuxiliarFaena;
GO

USE AuxiliarFaena;
GO

CREATE SCHEMA cambiarEtiquetas;
GO


SET ANSI_NULLS ON
GO
SET QUOTED_IDENTIFIER ON
GO


CREATE TABLE [cambiarEtiquetas].[FaenaEtiquetas](
	[id] [tinyint] IDENTITY(1,1) NOT NULL,
	[enable] [bit] NOT NULL,
	[etiqueta] [varchar](200) NOT NULL,
	[label] [varchar](16) NOT NULL,
	[color] [varchar](7) NOT NULL,
    CONSTRAINT [PK_FaenaEtiquetas] PRIMARY KEY CLUSTERED 
    ([id] ASC)
    WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, IGNORE_DUP_KEY = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON) ON [PRIMARY]
) ON [PRIMARY]
GO

SET ANSI_PADDING ON
GO

CREATE UNIQUE NONCLUSTERED INDEX [IX_Unique_FaenaEtiquetas] ON [cambiarEtiquetas].[FaenaEtiquetas]
([etiqueta] ASC) 
WITH (PAD_INDEX = OFF, STATISTICS_NORECOMPUTE = OFF, SORT_IN_TEMPDB = OFF, IGNORE_DUP_KEY = OFF, DROP_EXISTING = OFF, ONLINE = OFF, ALLOW_ROW_LOCKS = ON, ALLOW_PAGE_LOCKS = ON) 
ON [PRIMARY]
GO


INSERT INTO [cambiarEtiquetas].[FaenaEtiquetas]
VALUES 
    (1, 'faena_h_aa_2copias', 'H AA 2', '#664980'),
    (1, 'faena_h_aa_3copias', 'H AA 3', '#4C709A'),
    (1, 'faena_h_aa_4copias', 'H AA 4', '#567556'),
    (1, 'faena_hilton_2copias', 'Hilton 2', '#B7AA5E'),
    (1, 'faena_hilton_3copias', 'Hilton 3', '#A66A40'),
    (1, 'faena_hilton_4copias', 'Hilton 4', '#9B4244')
GO


CREATE  FUNCTION [cambiarEtiquetas].[SplitToList] ( @List varchar(MAX) )
RETURNS @ParsedList TABLE (item int)
AS
BEGIN
    DECLARE @item varchar(800), @Pos int

    SET @List = LTRIM(RTRIM(@List))+ ','
    SET @Pos = CHARINDEX(',', @List, 1)

    WHILE @Pos > 0
    BEGIN
        SET @item = LTRIM(RTRIM(LEFT(@List, @Pos - 1)))
        IF @item <> ''
        BEGIN
            INSERT INTO @ParsedList (item) 
            VALUES (CAST(@item AS int))
        END
        SET @List = RIGHT(@List, LEN(@List) - @Pos)
        SET @Pos = CHARINDEX(',', @List, 1)
    END

    RETURN
END
GO


CREATE PROCEDURE [cambiarEtiquetas].[ListarMedias]
	@mercaderias varchar(MAX) output
AS
BEGIN
	SELECT STRING_AGG(Id, ',')
	FROM [TwinsDBQuatro053].[configuracion].[Mercaderias]
	WHERE MercaderiaTipo_Id = 1 -- 1 es la id de Media
END
GO


/*
 *  Nombre: Cambiar Etiquetas
 *  Descripcion: Modifica la etiqueta de caja que utiliza la mercaderia especificada por parametro.
 *
 *  Proyecto: Faena Etiquetas
 *  Autor: Agustin Marco <agustin.marco@runfo.com.ar>
 *  Fecha: 04-03-2024
 *
 *  Parametros:
 *      @mercaderias --> Lista de mercaderias por actualizar [Falla si es null o vacio].
 *      @etiqueta --> Nombre de la etiqueta a usar en las mercaderias [Falla si es null o vacio].
 *      @prueba --> 0 habilita el modo prueba, 1 habilita el modo producción
 *
 *  Error Code: 56450
 *       Causa: No se ha podido actualizar la mercaderia.
 *  Error Code: 58450
 *       Causa: No existe el producto.
 *   Warn Code: 57450
 *       Causa: Se intento actualizar la mercaderia con una etiqueta no habilitada.
 */
CREATE PROCEDURE [cambiarEtiquetas].[CambiarEtiquetas]
	@mercaderias varchar(250) = '',
	@etiqueta varchar(60) = '',
    @prueba BIT = 0
AS
BEGIN
	SET NOCOUNT OFF;

    DECLARE @enable BIT --> Estado de la etiqueta
    DECLARE @faltantes INT --> Cantidad de productos que no se pudieron actualizar
    DECLARE @list TABLE(item INT) --> Lista de productos por ID
    DECLARE @err_msg NVARCHAR(MAX) --> Mensaje de error 56450
    DECLARE @warn_msg NVARCHAR(200) --> Mensaje de advertencia 57450
    DECLARE @mercaderia_activa TABLE(id INT) --> Lista de mercaderia activa para actualizar
    
    -- Paramos el proceso si alguna de los parametros esta vacio --
	IF (@mercaderias is null or @mercaderias = '')
        RAISERROR('La variable @mercaderia esta vacia', 11, 1)

	IF (@etiqueta is null or @etiqueta = '')
		RAISERROR('La variable @etiqueta esta vacia', 11, 2)

    -- Parseamos la lista de ids a una tabla --
    INSERT INTO @list SELECT * FROM [cambiarEtiquetas].[SplitToList](@mercaderias)
    
    -- Revisamos si la etiqueta esta habilitada para produccion --
    SET @enable = (SELECT [enable] FROM [cambiarEtiquetas].[FaenaEtiquetas] WHERE [etiqueta] = @etiqueta)
    IF (@enable = 0)
    BEGIN
        SET @warn_msg = 'Etiqueta ' + @etiqueta + ' no habilitada para producción.';
        THROW 57450, @warn_msg, 3;
    END

    -- Filtra la lista de mercaderia para solo las activas --
    INSERT INTO @mercaderia_activa SELECT Mercaderia.Id
    FROM [TwinsDBQuatro053].[configuracion].[Mercaderias] as Mercaderia 
        INNER JOIN [TwinsDBQuatro053].[configuracion].[MercaderiasEtiquetaCaja] as EtiquetasCaja
        ON Mercaderia.Id = EtiquetasCaja.Mercaderia_Id
    WHERE Mercaderia.Id in (SELECT * FROM @list) AND Mercaderia.bActivo = 1

    -- Genera una falla si no existe ninguna mercaderia --
    IF NOT EXISTS (SELECT 1 FROM @mercaderia_activa)
        BEGIN
        SET @err_msg = 'Mercaderia (' + @mercaderias + ') no existe';
        THROW 58450, @err_msg, 4;
    END

    -- Update query para actualizar las etiquetas, filtra por mercaderia activa --
    IF @prueba = 1
	    UPDATE [TwinsDBQuatro053].[configuracion].[MercaderiasEtiquetaCaja] SET sEtiqueta = @etiqueta
        WHERE Mercaderia_Id in (SELECT * FROM @mercaderia_activa)
    ELSE
    BEGIN
        PRINT 'Cambiar Etiquetas (modo prueba)'

        SELECT *
        FROM [TwinsDBQuatro053].[configuracion].[MercaderiasEtiquetaCaja]
        WHERE Mercaderia_Id in (SELECT * FROM @mercaderia_activa)

        RETURN -- No hace falta continuar con el proceso en modo prueba --
    END

    -- Selecciona las mercaderias que no se han podido actualizar --
    SELECT @faltantes = COUNT(*), @err_msg = STRING_AGG(Mercaderia_Id, ',')
    FROM [TwinsDBQuatro053].[configuracion].[MercaderiasEtiquetaCaja]
    WHERE Mercaderia_Id in (SELECT * FROM @mercaderia_activa) AND sEtiqueta <> @etiqueta

    -- Genera una falla si no se modifico ninguna mercaderia --
    IF @faltantes = (SELECT COUNT(*) FROM @list)
    BEGIN
        SET @err_msg = 'No se han podido actualizar esta mercaderia: ' + @err_msg;
        THROW 56450, @err_msg, 4;
    END
END
GO


/*** Pruebas ***/
DECLARE @mercaderias varchar(250) = '11,12,13,14,15,16,17,18,19,95,398,399,400,401,402,403,404,405,406'
DECLARE @etiqueta varchar(60) = 'faena_h_aa_4copias'


PRINT 'Prueba 1: Falta parametros'
BEGIN TRY
    EXECUTE [cambiarEtiquetas].[CambiarEtiquetas] @mercaderias
END TRY
BEGIN CATCH
    SELECT ERROR_NUMBER() AS ErrorNumber, ERROR_MESSAGE() AS ErrorMessage
END CATCH


PRINT 'Prueba 2: Etiqueta no habilitada'
BEGIN TRY
    UPDATE [cambiarEtiquetas].[FaenaEtiquetas] SET enable = 0 WHERE etiqueta = @etiqueta
    EXECUTE [cambiarEtiquetas].[CambiarEtiquetas] @mercaderias, @etiqueta
END TRY
BEGIN CATCH
    SELECT ERROR_NUMBER() AS ErrorNumber, ERROR_MESSAGE() AS ErrorMessage
END CATCH
UPDATE [cambiarEtiquetas].[FaenaEtiquetas] SET enable = 1 WHERE etiqueta = @etiqueta


PRINT 'Prueba 3: Mercaderia no existe'
BEGIN TRY
    EXECUTE [cambiarEtiquetas].[CambiarEtiquetas] '-1', @etiqueta, 1
END TRY
BEGIN CATCH
    SELECT ERROR_NUMBER() AS ErrorNumber, ERROR_MESSAGE() AS ErrorMessage
END CATCH


PRINT 'Prueba 4: Modo Prueba'
EXECUTE [cambiarEtiquetas].[CambiarEtiquetas] @mercaderias, @etiqueta

GO